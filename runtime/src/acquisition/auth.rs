//! Authentication handlers -- login via password, OAuth, API key.
//!
//! Discovers login methods from HTML, executes password-based login via HTTP
//! (no browser needed for standard forms), and creates authenticated sessions.

use crate::acquisition::http_client::HttpClient;
use crate::acquisition::http_session::{AuthType, HttpSession};
use anyhow::{bail, Result};
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// ---- Public types -----------------------------------------------------------

/// Discovered login method for a site.
#[derive(Debug, Clone)]
pub enum LoginMethod {
    /// Password form found at a URL.
    Password {
        /// URL of the page containing the login form.
        form_url: String,
        /// Resolved action URL where the form POSTs to.
        form_action: String,
        /// HTTP method (usually `POST`).
        method: String,
        /// All fields in the login form.
        fields: Vec<LoginFormField>,
    },
    /// OAuth providers detected.
    OAuth {
        /// Names of detected OAuth providers (e.g. `"google"`, `"github"`).
        providers: Vec<String>,
    },
    /// API key documentation found.
    ApiKey {
        /// URL to API key documentation, if discovered.
        docs_url: Option<String>,
    },
    /// Could not determine login method.
    Unknown,
}

/// A field in a login form.
#[derive(Debug, Clone)]
pub struct LoginFormField {
    /// The `name` attribute of the input element.
    pub name: String,
    /// The `type` attribute (e.g. `"text"`, `"password"`, `"hidden"`).
    pub field_type: String,
    /// Pre-filled value (for hidden fields like CSRF tokens).
    pub value: Option<String>,
    /// Whether this field is the username/email field.
    pub is_username: bool,
    /// Whether this field is the password field.
    pub is_password: bool,
}

// ---- OAuth types ------------------------------------------------------------

/// Result of an HTTP-native OAuth flow.
///
/// OAuth redirect chains can often be completed without a browser when consent
/// was previously granted. When consent *is* needed, the flow pauses for
/// agent/user approval.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum OAuthResult {
    /// Consent was pre-approved — auth code obtained via redirect chain.
    SilentSuccess {
        /// The authorization code.
        code: String,
        /// The final redirect URL containing the code.
        redirect_url: String,
    },
    /// Consent is needed — pausing for user/agent approval.
    ConsentRequired {
        /// OAuth scopes requested.
        scopes: Vec<String>,
        /// Application name requesting access.
        app_name: String,
        /// The parsed consent form for submission.
        consent_form: ConsentForm,
    },
    /// Multi-factor authentication required during auth flow.
    MfaRequired {
        /// Type of MFA challenge.
        mfa_type: MfaType,
        /// The MFA challenge form.
        challenge_form: HtmlForm,
    },
    /// HTTP approach failed — fall back to browser.
    BrowserFallbackNeeded {
        /// Reason the HTTP approach failed.
        reason: String,
    },
}

/// A parsed consent form from an OAuth provider.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConsentForm {
    /// The URL to POST consent approval to.
    pub action_url: String,
    /// Hidden fields required for the POST (state, CSRF, etc.).
    pub hidden_fields: HashMap<String, String>,
    /// The OAuth provider name.
    pub provider: String,
}

/// Type of MFA challenge.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MfaType {
    /// Time-based one-time password (TOTP / authenticator app).
    Totp,
    /// SMS verification code.
    Sms,
    /// Email verification code.
    Email,
    /// Push notification (e.g., Duo).
    Push,
    /// Unknown MFA type.
    Unknown,
}

/// A parsed HTML form with action URL and fields.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HtmlForm {
    /// The URL the form posts to.
    pub action_url: String,
    /// HTTP method (usually POST).
    pub method: String,
    /// All form fields: name → value.
    pub fields: HashMap<String, String>,
}

// ---- Public async API -------------------------------------------------------

/// Discover the login method for a site by fetching its homepage and login page.
///
/// Checks common login URL patterns (`/login`, `/signin`, `/auth`,
/// `/account/login`, `/wp-login.php`). When a login page is found, its HTML
/// is analysed for password forms and OAuth buttons.
pub async fn discover_login_method(client: &HttpClient, domain: &str) -> Result<LoginMethod> {
    let base_url = format!("https://{domain}");

    // Fetch the homepage and look for login links.
    let homepage = client.get(&base_url, 15_000).await?;
    let login_links = find_login_links(&homepage.body, &base_url);

    // Also try well-known login paths even if no link was found.
    let well_known = [
        format!("{base_url}/login"),
        format!("{base_url}/signin"),
        format!("{base_url}/auth/login"),
        format!("{base_url}/account/login"),
        format!("{base_url}/wp-login.php"),
    ];

    // Merge discovered links with well-known paths, deduplicated.
    let mut candidates: Vec<String> = login_links;
    for wk in &well_known {
        if !candidates.contains(wk) {
            candidates.push(wk.clone());
        }
    }

    // Try each candidate until we find a login form.
    for candidate_url in &candidates {
        let resp = match client.get(candidate_url, 15_000).await {
            Ok(r) => r,
            Err(_) => continue,
        };

        // Skip non-200 responses.
        if resp.status != 200 {
            continue;
        }

        if let Some(method) = parse_login_form(&resp.body, &resp.final_url) {
            return Ok(method);
        }
    }

    // If no form found, check homepage itself for OAuth buttons.
    if let Some(LoginMethod::OAuth { providers }) =
        detect_oauth_from_html(&homepage.body).filter(|m| matches!(m, LoginMethod::OAuth { .. }))
    {
        if !providers.is_empty() {
            return Ok(LoginMethod::OAuth { providers });
        }
    }

    Ok(LoginMethod::Unknown)
}

/// Log in with username and password via HTML form POST.
///
/// Discovers the login form, fills in the username and password fields (keeping
/// hidden fields like CSRF tokens), POSTs the form, and captures session
/// cookies from `Set-Cookie` response headers.
pub async fn login_password(
    client: &HttpClient,
    domain: &str,
    username: &str,
    password: &str,
) -> Result<HttpSession> {
    let method = discover_login_method(client, domain).await?;

    let (form_action, http_method, fields) = match method {
        LoginMethod::Password {
            form_action,
            method,
            fields,
            ..
        } => (form_action, method, fields),
        _ => bail!("no password login form found for {domain}"),
    };

    // Build form body: fill in username/password, keep hidden fields.
    let mut form_data: Vec<(String, String)> = Vec::new();
    for field in &fields {
        if field.is_username {
            form_data.push((field.name.clone(), username.to_string()));
        } else if field.is_password {
            form_data.push((field.name.clone(), password.to_string()));
        } else if let Some(ref val) = field.value {
            form_data.push((field.name.clone(), val.clone()));
        }
    }

    // Extract CSRF token if present.
    let csrf_token = fields
        .iter()
        .find(|f| is_csrf_field_name(&f.name))
        .and_then(|f| f.value.clone());

    // POST the form.
    if http_method != "POST" {
        bail!("login form uses {http_method}, expected POST");
    }

    let resp = client
        .post_form(&form_action, &form_data, &[], 15_000)
        .await?;

    // Parse Set-Cookie headers.
    let cookies = parse_set_cookies(&resp.headers);

    if cookies.is_empty() && resp.status >= 400 {
        bail!(
            "login failed for {domain}: status {} with no cookies",
            resp.status
        );
    }

    let mut session = HttpSession::new(domain, AuthType::Password);
    for (name, value) in cookies {
        session.add_cookie(&name, &value);
    }
    session.csrf_token = csrf_token;

    Ok(session)
}

/// Create an API-key authenticated session (no network call needed).
///
/// The key is stored as an auth header with the given name (e.g. `X-Api-Key`).
pub fn login_api_key(domain: &str, key: &str, header_name: &str) -> HttpSession {
    let mut session = HttpSession::new(domain, AuthType::ApiKey);
    session.add_auth_header(header_name, key);
    session
}

/// Create a bearer-token authenticated session (no network call needed).
///
/// Stores `Authorization: Bearer {token}` as an auth header.
pub fn login_bearer(domain: &str, token: &str) -> HttpSession {
    let mut session = HttpSession::new(domain, AuthType::Bearer);
    session.add_auth_header("Authorization", &format!("Bearer {token}"));
    session
}

// ---- OAuth HTTP flow --------------------------------------------------------

/// Attempt OAuth login via HTTP redirect chain (no browser).
///
/// Follows the OAuth redirect chain via HTTP. If consent was previously
/// granted, the provider redirects directly to the callback with an auth
/// code — zero browser, sub-second. If consent is needed, parses the
/// consent page HTML and returns `OAuthResult::ConsentRequired`.
///
/// # Arguments
///
/// * `client` - HTTP client for making requests.
/// * `auth_url` - The initial OAuth authorization URL (e.g., from the login page).
/// * `provider` - OAuth provider name (e.g., `"google"`, `"github"`).
pub async fn login_oauth_http(
    client: &HttpClient,
    auth_url: &str,
    provider: &str,
) -> Result<OAuthResult> {
    // Step 1: Follow redirects via HTTP GET.
    // We use the client to follow the initial redirect chain.
    let resp = client.get(auth_url, 15_000).await?;

    // Check if we got redirected to a callback URL with an auth code.
    // This happens when consent was previously granted.
    if let Some(code) = extract_auth_code_from_url(&resp.final_url) {
        tracing::info!("OAuth silent success for {provider}: consent was pre-approved");
        return Ok(OAuthResult::SilentSuccess {
            code,
            redirect_url: resp.final_url,
        });
    }

    // Check for MFA challenge pages.
    if is_mfa_page(&resp.body) {
        let mfa_type = detect_mfa_type(&resp.body);
        let form = parse_first_form(&resp.body, &resp.final_url);
        return Ok(match form {
            Some(f) => OAuthResult::MfaRequired {
                mfa_type,
                challenge_form: f,
            },
            None => OAuthResult::BrowserFallbackNeeded {
                reason: "MFA page found but could not parse form".to_string(),
            },
        });
    }

    // Step 2: If we're on a consent page, parse it.
    if resp.status == 200 && is_consent_page(&resp.body) {
        let scopes = extract_oauth_scopes(&resp.body);
        let app_name = extract_app_name(&resp.body);
        let consent_form = parse_consent_form(&resp.body, &resp.final_url, provider);

        return Ok(match consent_form {
            Some(form) => OAuthResult::ConsentRequired {
                scopes,
                app_name,
                consent_form: form,
            },
            None => OAuthResult::BrowserFallbackNeeded {
                reason: "consent page found but could not parse approval form".to_string(),
            },
        });
    }

    // Step 3: If the response is an error or unknown page, fall back to browser.
    if resp.status >= 400 {
        return Ok(OAuthResult::BrowserFallbackNeeded {
            reason: format!("OAuth redirect returned status {}", resp.status),
        });
    }

    // Unknown page — might need browser interaction.
    Ok(OAuthResult::BrowserFallbackNeeded {
        reason: "could not complete OAuth flow via HTTP".to_string(),
    })
}

/// Complete an OAuth consent form by POSTing approval.
///
/// Called after the agent/user approves the consent request.
///
/// Returns the authorization code on success.
pub async fn complete_oauth_consent(
    client: &HttpClient,
    consent_form: &ConsentForm,
    approved: bool,
) -> Result<String> {
    if !approved {
        bail!("OAuth consent was denied by user/agent");
    }

    // Build form data from hidden fields + approval flag.
    let mut form_data: Vec<(String, String)> = consent_form
        .hidden_fields
        .iter()
        .map(|(k, v)| (k.clone(), v.clone()))
        .collect();
    form_data.push(("submit_access".to_string(), "true".to_string()));

    let resp = client
        .post_form(&consent_form.action_url, &form_data, &[], 15_000)
        .await?;

    // Check if the response redirected to a callback URL with an auth code.
    if let Some(code) = extract_auth_code_from_url(&resp.final_url) {
        return Ok(code);
    }

    // Try to find the code in the response body (some providers embed it).
    if let Some(code) = extract_auth_code_from_body(&resp.body) {
        return Ok(code);
    }

    bail!(
        "OAuth consent submission did not yield an auth code (status: {})",
        resp.status
    )
}

/// Submit an MFA code via form POST.
///
/// Returns an updated session with cookies from the MFA response.
pub async fn handle_oauth_mfa(
    client: &HttpClient,
    form: &HtmlForm,
    mfa_code: &str,
    domain: &str,
) -> Result<HttpSession> {
    let mut form_data: Vec<(String, String)> = form
        .fields
        .iter()
        .map(|(k, v)| (k.clone(), v.clone()))
        .collect();

    // Common MFA field names.
    let mfa_field_names = ["code", "otp", "mfa_code", "verification_code", "pin"];
    let mfa_field = form
        .fields
        .keys()
        .find(|k| {
            let lower = k.to_lowercase();
            mfa_field_names.iter().any(|n| lower.contains(n))
        })
        .cloned();

    if let Some(field_name) = mfa_field {
        // Update the existing field.
        for (k, v) in &mut form_data {
            if k == &field_name {
                *v = mfa_code.to_string();
            }
        }
    } else {
        // Use "code" as default field name.
        form_data.push(("code".to_string(), mfa_code.to_string()));
    }

    let resp = client
        .post_form(&form.action_url, &form_data, &[], 15_000)
        .await?;

    let cookies = parse_set_cookies(&resp.headers);
    if cookies.is_empty() && resp.status >= 400 {
        bail!("MFA verification failed: status {}", resp.status);
    }

    let mut session = HttpSession::new(domain, AuthType::OAuth("mfa".to_string()));
    for (name, value) in cookies {
        session.add_cookie(&name, &value);
    }

    Ok(session)
}

// ---- Private helpers --------------------------------------------------------

/// Scan HTML for `<a>` tags whose href contains login-related paths.
fn find_login_links(html: &str, base_url: &str) -> Vec<String> {
    let link_re =
        Regex::new(r#"<a\s[^>]*href\s*=\s*["']([^"']+)["'][^>]*>"#).expect("link regex is valid");

    let login_patterns = [
        "/login",
        "/signin",
        "/sign-in",
        "/auth",
        "/account/login",
        "/wp-login.php",
        "/users/sign_in",
        "/session/new",
    ];

    let mut found = Vec::new();
    for caps in link_re.captures_iter(html) {
        let href = caps.get(1).map_or("", |m| m.as_str());
        let href_lower = href.to_lowercase();
        if login_patterns.iter().any(|p| href_lower.contains(p)) {
            let resolved = resolve_url(base_url, href);
            if !found.contains(&resolved) {
                found.push(resolved);
            }
        }
    }
    found
}

/// Parse HTML to find a login form (a `<form>` with a password input).
///
/// Returns `LoginMethod::Password` if a suitable form is found, `None` otherwise.
fn parse_login_form(html: &str, base_url: &str) -> Option<LoginMethod> {
    // Find <form> blocks that contain a password input.
    let form_re = Regex::new(r"(?is)<form\b([^>]*)>(.*?)</form>").expect("form regex is valid");
    let action_re =
        Regex::new(r#"(?i)action\s*=\s*["']([^"']+)["']"#).expect("action regex is valid");
    let method_re =
        Regex::new(r#"(?i)method\s*=\s*["']([^"']+)["']"#).expect("method regex is valid");
    let input_re = Regex::new(r#"(?i)<input\b([^>]*)>"#).expect("input regex is valid");
    let attr_name_re =
        Regex::new(r#"(?i)name\s*=\s*["']([^"']+)["']"#).expect("attr name regex is valid");
    let attr_type_re =
        Regex::new(r#"(?i)type\s*=\s*["']([^"']+)["']"#).expect("attr type regex is valid");
    let attr_value_re =
        Regex::new(r#"(?i)value\s*=\s*["']([^"']*?)["']"#).expect("attr value regex is valid");

    for form_caps in form_re.captures_iter(html) {
        let form_attrs = form_caps.get(1).map_or("", |m| m.as_str());
        let form_body = form_caps.get(2).map_or("", |m| m.as_str());

        // Only consider forms that contain a password field.
        if !form_body.to_lowercase().contains("type=\"password\"")
            && !form_body.to_lowercase().contains("type='password'")
        {
            continue;
        }

        let form_action = action_re
            .captures(form_attrs)
            .and_then(|c| c.get(1))
            .map(|m| resolve_url(base_url, m.as_str()))
            .unwrap_or_else(|| base_url.to_string());

        let method = method_re
            .captures(form_attrs)
            .and_then(|c| c.get(1))
            .map(|m| m.as_str().to_uppercase())
            .unwrap_or_else(|| "POST".to_string());

        let mut fields = Vec::new();
        for input_caps in input_re.captures_iter(form_body) {
            let input_attrs = input_caps.get(1).map_or("", |m| m.as_str());

            let name = match attr_name_re.captures(input_attrs) {
                Some(c) => c.get(1).map_or("", |m| m.as_str()).to_string(),
                None => continue, // skip inputs without a name
            };

            let field_type = attr_type_re
                .captures(input_attrs)
                .and_then(|c| c.get(1))
                .map(|m| m.as_str().to_lowercase())
                .unwrap_or_else(|| "text".to_string());

            let value = attr_value_re
                .captures(input_attrs)
                .and_then(|c| c.get(1))
                .map(|m| m.as_str().to_string());

            let is_password = field_type == "password";
            let is_username = !is_password
                && (field_type == "text" || field_type == "email")
                && is_username_field_name(&name);

            fields.push(LoginFormField {
                name,
                field_type,
                value,
                is_username,
                is_password,
            });
        }

        // If we didn't identify any username field, pick the first text/email
        // field that is not a CSRF token.
        let has_username = fields.iter().any(|f| f.is_username);
        if !has_username {
            if let Some(f) = fields.iter_mut().find(|f| {
                (f.field_type == "text" || f.field_type == "email") && !is_csrf_field_name(&f.name)
            }) {
                f.is_username = true;
            }
        }

        return Some(LoginMethod::Password {
            form_url: base_url.to_string(),
            form_action,
            method,
            fields,
        });
    }

    // Check for OAuth-only login pages.
    detect_oauth_from_html(html)
}

/// Detect OAuth providers from HTML content.
fn detect_oauth_from_html(html: &str) -> Option<LoginMethod> {
    let mut providers = Vec::new();

    let oauth_patterns: &[(&str, &str)] = &[
        ("accounts.google.com", "google"),
        ("github.com/login/oauth", "github"),
        ("facebook.com/v", "facebook"),
        ("login.microsoftonline.com", "microsoft"),
        ("appleid.apple.com", "apple"),
        ("twitter.com/oauth", "twitter"),
        ("api.twitter.com/oauth", "twitter"),
    ];

    let html_lower = html.to_lowercase();
    for (pattern, provider) in oauth_patterns {
        if html_lower.contains(pattern) && !providers.contains(&provider.to_string()) {
            providers.push(provider.to_string());
        }
    }

    if providers.is_empty() {
        None
    } else {
        Some(LoginMethod::OAuth { providers })
    }
}

/// Check if a field name looks like a username/email field.
fn is_username_field_name(name: &str) -> bool {
    let lower = name.to_lowercase();
    lower.contains("user")
        || lower.contains("email")
        || lower.contains("login")
        || lower.contains("account")
        || lower == "id"
        || lower == "name"
        || lower == "username"
}

/// Check if a field name looks like a CSRF token.
fn is_csrf_field_name(name: &str) -> bool {
    let lower = name.to_lowercase();
    lower.contains("csrf")
        || lower.contains("_token")
        || lower == "authenticity_token"
        || lower.contains("nonce")
        || lower.contains("xsrf")
}

/// Parse `Set-Cookie` headers into name-value pairs.
///
/// Each `Set-Cookie` header has the form `name=value; attr1; attr2=val2`.
/// Only the `name=value` portion is extracted.
fn parse_set_cookies(headers: &[(String, String)]) -> HashMap<String, String> {
    let mut cookies = HashMap::new();

    for (name, value) in headers {
        if name.to_lowercase() != "set-cookie" {
            continue;
        }

        // The cookie is everything before the first `;`.
        let cookie_part = value.split(';').next().unwrap_or("");
        if let Some(eq_pos) = cookie_part.find('=') {
            let cname = cookie_part[..eq_pos].trim().to_string();
            let cvalue = cookie_part[eq_pos + 1..].trim().to_string();
            if !cname.is_empty() {
                cookies.insert(cname, cvalue);
            }
        }
    }

    cookies
}

/// Resolve a potentially relative URL against a base URL.
fn resolve_url(base_url: &str, relative: &str) -> String {
    if relative.is_empty() {
        return base_url.to_string();
    }
    if relative.starts_with("http://") || relative.starts_with("https://") {
        return relative.to_string();
    }
    if let Ok(base) = url::Url::parse(base_url) {
        if let Ok(resolved) = base.join(relative) {
            return resolved.to_string();
        }
    }
    relative.to_string()
}

// ---- OAuth private helpers --------------------------------------------------

/// Extract an authorization code from a redirect URL's query parameters.
fn extract_auth_code_from_url(url: &str) -> Option<String> {
    let parsed = url::Url::parse(url).ok()?;
    parsed
        .query_pairs()
        .find(|(k, _)| k == "code")
        .map(|(_, v)| v.to_string())
}

/// Extract an authorization code from an HTML response body.
fn extract_auth_code_from_body(body: &str) -> Option<String> {
    let code_re = Regex::new(r#"code['"]\s*(?:value|content)\s*=\s*['"]([^'"]+)['"]"#).ok()?;
    code_re
        .captures(body)
        .and_then(|c| c.get(1))
        .map(|m| m.as_str().to_string())
}

/// Check if an HTML page looks like an OAuth consent page.
fn is_consent_page(html: &str) -> bool {
    let lower = html.to_lowercase();
    (lower.contains("consent") || lower.contains("authorize") || lower.contains("grant access"))
        && (lower.contains("scope") || lower.contains("permission"))
}

/// Check if an HTML page looks like an MFA challenge page.
fn is_mfa_page(html: &str) -> bool {
    let lower = html.to_lowercase();
    (lower.contains("verification")
        || lower.contains("2-step")
        || lower.contains("two-factor")
        || lower.contains("mfa")
        || lower.contains("authenticator"))
        && (lower.contains("<form") || lower.contains("<input"))
}

/// Detect the type of MFA challenge from page content.
fn detect_mfa_type(html: &str) -> MfaType {
    let lower = html.to_lowercase();
    if lower.contains("authenticator") || lower.contains("totp") || lower.contains("6-digit") {
        MfaType::Totp
    } else if lower.contains("sms") || lower.contains("text message") || lower.contains("phone") {
        MfaType::Sms
    } else if lower.contains("email") && lower.contains("code") {
        MfaType::Email
    } else if lower.contains("push") || lower.contains("notification") || lower.contains("duo") {
        MfaType::Push
    } else {
        MfaType::Unknown
    }
}

/// Extract OAuth scopes from a consent page.
fn extract_oauth_scopes(html: &str) -> Vec<String> {
    let mut scopes = Vec::new();

    // Look for scope items in list elements.
    let scope_re = Regex::new(r#"(?i)<li[^>]*class="[^"]*scope[^"]*"[^>]*>([^<]+)</li>"#).unwrap();
    for caps in scope_re.captures_iter(html) {
        if let Some(m) = caps.get(1) {
            let scope = m.as_str().trim().to_string();
            if !scope.is_empty() && !scopes.contains(&scope) {
                scopes.push(scope);
            }
        }
    }

    // Fallback: look for common scope keywords.
    if scopes.is_empty() {
        let scope_keywords = ["email", "profile", "openid", "read", "write"];
        let lower = html.to_lowercase();
        for keyword in &scope_keywords {
            if lower.contains(keyword) {
                scopes.push(keyword.to_string());
            }
        }
    }

    scopes
}

/// Extract the application name from a consent page.
fn extract_app_name(html: &str) -> String {
    // Try to find app name in common patterns.
    let app_re =
        Regex::new(r#"(?i)(?:<strong>|<b>|class="[^"]*app[_-]?name[^"]*"[^>]*>)([^<]+)<"#).unwrap();

    if let Some(caps) = app_re.captures(html) {
        if let Some(m) = caps.get(1) {
            return m.as_str().trim().to_string();
        }
    }

    "Unknown Application".to_string()
}

/// Parse a consent form from an OAuth consent page.
fn parse_consent_form(html: &str, base_url: &str, provider: &str) -> Option<ConsentForm> {
    let form = parse_first_form(html, base_url)?;

    Some(ConsentForm {
        action_url: form.action_url,
        hidden_fields: form.fields,
        provider: provider.to_string(),
    })
}

/// Parse the first `<form>` tag from HTML into an HtmlForm.
fn parse_first_form(html: &str, base_url: &str) -> Option<HtmlForm> {
    let form_re = Regex::new(r"(?is)<form\b([^>]*)>(.*?)</form>").ok()?;
    let action_re = Regex::new(r#"(?i)action\s*=\s*["']([^"']+)["']"#).ok()?;
    let method_re = Regex::new(r#"(?i)method\s*=\s*["']([^"']+)["']"#).ok()?;
    let input_re = Regex::new(r#"(?i)<input\b([^>]*)>"#).ok()?;
    let name_re = Regex::new(r#"(?i)name\s*=\s*["']([^"']+)["']"#).ok()?;
    let value_re = Regex::new(r#"(?i)value\s*=\s*["']([^"']*?)["']"#).ok()?;

    let form_caps = form_re.captures(html)?;
    let form_attrs = form_caps.get(1).map_or("", |m| m.as_str());
    let form_body = form_caps.get(2).map_or("", |m| m.as_str());

    let action_url = action_re
        .captures(form_attrs)
        .and_then(|c| c.get(1))
        .map(|m| resolve_url(base_url, m.as_str()))
        .unwrap_or_else(|| base_url.to_string());

    let method = method_re
        .captures(form_attrs)
        .and_then(|c| c.get(1))
        .map(|m| m.as_str().to_uppercase())
        .unwrap_or_else(|| "POST".to_string());

    let mut fields = HashMap::new();
    for input_caps in input_re.captures_iter(form_body) {
        let input_attrs = input_caps.get(1).map_or("", |m| m.as_str());
        if let Some(name_cap) = name_re.captures(input_attrs) {
            let name = name_cap.get(1).map_or("", |m| m.as_str()).to_string();
            let value = value_re
                .captures(input_attrs)
                .and_then(|c| c.get(1))
                .map(|m| m.as_str().to_string())
                .unwrap_or_default();
            if !name.is_empty() {
                fields.insert(name, value);
            }
        }
    }

    Some(HtmlForm {
        action_url,
        method,
        fields,
    })
}

// ---- Tests ------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_find_login_links() {
        let html = r#"
        <html><body>
            <a href="/about">About</a>
            <a href="/login">Log In</a>
            <a href="/products">Products</a>
            <a href="/account/login">My Account</a>
        </body></html>
        "#;

        let links = find_login_links(html, "https://example.com");
        assert_eq!(links.len(), 2);
        assert!(links.contains(&"https://example.com/login".to_string()));
        assert!(links.contains(&"https://example.com/account/login".to_string()));
    }

    #[test]
    fn test_parse_login_form() {
        let html = r#"
        <html><body>
            <form action="/auth/login" method="POST">
                <input type="hidden" name="csrf_token" value="abc123" />
                <input type="email" name="email" />
                <input type="password" name="password" />
                <button type="submit">Sign In</button>
            </form>
        </body></html>
        "#;

        let method = parse_login_form(html, "https://example.com");
        assert!(method.is_some());

        if let Some(LoginMethod::Password {
            form_action,
            method,
            fields,
            ..
        }) = method
        {
            assert_eq!(form_action, "https://example.com/auth/login");
            assert_eq!(method, "POST");
            assert_eq!(fields.len(), 3);

            let csrf = fields.iter().find(|f| f.name == "csrf_token").unwrap();
            assert_eq!(csrf.field_type, "hidden");
            assert_eq!(csrf.value.as_deref(), Some("abc123"));
            assert!(!csrf.is_username);
            assert!(!csrf.is_password);

            let email = fields.iter().find(|f| f.name == "email").unwrap();
            assert!(email.is_username);
            assert!(!email.is_password);

            let pw = fields.iter().find(|f| f.name == "password").unwrap();
            assert!(!pw.is_username);
            assert!(pw.is_password);
        } else {
            panic!("expected LoginMethod::Password");
        }
    }

    #[test]
    fn test_parse_login_form_oauth() {
        let html = r#"
        <html><body>
            <a href="https://accounts.google.com/o/oauth2/auth?client_id=123">
                Sign in with Google
            </a>
            <a href="https://github.com/login/oauth/authorize?client_id=456">
                Sign in with GitHub
            </a>
        </body></html>
        "#;

        let method = parse_login_form(html, "https://example.com");
        assert!(method.is_some());

        if let Some(LoginMethod::OAuth { providers }) = method {
            assert!(providers.contains(&"google".to_string()));
            assert!(providers.contains(&"github".to_string()));
        } else {
            panic!("expected LoginMethod::OAuth");
        }
    }

    #[test]
    fn test_login_api_key() {
        let session = login_api_key("api.example.com", "my-secret-key", "X-Api-Key");

        assert_eq!(session.domain, "api.example.com");
        assert_eq!(session.auth_type, AuthType::ApiKey);
        assert_eq!(
            session.auth_headers.get("X-Api-Key").unwrap(),
            "my-secret-key"
        );
        assert!(session.cookies.is_empty());
    }

    #[test]
    fn test_login_bearer() {
        let session = login_bearer("api.example.com", "tok_abc123");

        assert_eq!(session.domain, "api.example.com");
        assert_eq!(session.auth_type, AuthType::Bearer);
        assert_eq!(
            session.auth_headers.get("Authorization").unwrap(),
            "Bearer tok_abc123"
        );
        assert!(session.cookies.is_empty());
    }

    #[test]
    fn test_parse_set_cookies() {
        let headers = vec![
            ("content-type".to_string(), "text/html".to_string()),
            (
                "set-cookie".to_string(),
                "session_id=abc123; Path=/; HttpOnly".to_string(),
            ),
            (
                "set-cookie".to_string(),
                "csrftoken=xyz789; Secure; SameSite=Strict".to_string(),
            ),
            (
                "set-cookie".to_string(),
                "pref=dark; Max-Age=3600".to_string(),
            ),
        ];

        let cookies = parse_set_cookies(&headers);
        assert_eq!(cookies.len(), 3);
        assert_eq!(cookies.get("session_id").unwrap(), "abc123");
        assert_eq!(cookies.get("csrftoken").unwrap(), "xyz789");
        assert_eq!(cookies.get("pref").unwrap(), "dark");
    }

    #[test]
    fn test_find_login_links_absolute_url() {
        let html = r#"
        <html><body>
            <a href="https://auth.example.com/signin">Sign In</a>
        </body></html>
        "#;

        let links = find_login_links(html, "https://example.com");
        assert_eq!(links.len(), 1);
        assert_eq!(links[0], "https://auth.example.com/signin");
    }

    #[test]
    fn test_find_login_links_no_matches() {
        let html = r#"
        <html><body>
            <a href="/about">About</a>
            <a href="/products">Products</a>
        </body></html>
        "#;

        let links = find_login_links(html, "https://example.com");
        assert!(links.is_empty());
    }

    #[test]
    fn test_extract_auth_code_from_url() {
        let url = "https://example.com/callback?code=abc123&state=xyz";
        assert_eq!(extract_auth_code_from_url(url), Some("abc123".to_string()));

        let no_code = "https://example.com/callback?error=denied";
        assert_eq!(extract_auth_code_from_url(no_code), None);
    }

    #[test]
    fn test_is_consent_page() {
        let consent_html = r#"
        <html><body>
            <h1>ExampleApp wants to access your account</h1>
            <p>This app is requesting the following permissions (scope):</p>
            <ul><li>View your email</li><li>View your profile</li></ul>
            <form action="/consent" method="POST">
                <input type="hidden" name="state" value="abc" />
                <button name="submit_access" value="true">Grant access</button>
            </form>
        </body></html>
        "#;
        assert!(is_consent_page(consent_html));

        let normal_html = "<html><body><h1>Welcome</h1></body></html>";
        assert!(!is_consent_page(normal_html));
    }

    #[test]
    fn test_is_mfa_page() {
        let mfa_html = r#"
        <html><body>
            <h1>2-Step Verification</h1>
            <p>Enter the 6-digit code from your authenticator app</p>
            <form action="/verify" method="POST">
                <input type="text" name="code" />
                <button type="submit">Verify</button>
            </form>
        </body></html>
        "#;
        assert!(is_mfa_page(mfa_html));

        let normal = "<html><body><h1>Login</h1></body></html>";
        assert!(!is_mfa_page(normal));
    }

    #[test]
    fn test_detect_mfa_type() {
        assert!(matches!(
            detect_mfa_type("Enter the 6-digit code from your authenticator app"),
            MfaType::Totp
        ));
        assert!(matches!(
            detect_mfa_type("We sent a code via SMS to your phone"),
            MfaType::Sms
        ));
        assert!(matches!(
            detect_mfa_type("Check your email for a verification code"),
            MfaType::Email
        ));
        assert!(matches!(
            detect_mfa_type("Approve the push notification on your Duo app"),
            MfaType::Push
        ));
    }

    #[test]
    fn test_parse_consent_form() {
        let html = r#"
        <html><body>
            <form action="/oauth/approve" method="POST">
                <input type="hidden" name="state" value="xyz789" />
                <input type="hidden" name="client_id" value="app123" />
                <input type="hidden" name="scope" value="email profile" />
                <button name="submit_access" value="true">Allow</button>
            </form>
        </body></html>
        "#;

        let form = parse_consent_form(html, "https://accounts.example.com", "example");
        assert!(form.is_some());

        let f = form.unwrap();
        assert_eq!(f.action_url, "https://accounts.example.com/oauth/approve");
        assert_eq!(f.provider, "example");
        assert_eq!(f.hidden_fields.get("state").unwrap(), "xyz789");
        assert_eq!(f.hidden_fields.get("client_id").unwrap(), "app123");
    }

    #[test]
    fn test_parse_first_form() {
        let html = r#"
        <form action="/submit" method="POST">
            <input type="hidden" name="token" value="abc" />
            <input type="text" name="code" value="" />
        </form>
        "#;

        let form = parse_first_form(html, "https://example.com").unwrap();
        assert_eq!(form.action_url, "https://example.com/submit");
        assert_eq!(form.method, "POST");
        assert_eq!(form.fields.get("token").unwrap(), "abc");
        assert!(form.fields.contains_key("code"));
    }

    #[test]
    fn test_parse_login_form_no_password_field() {
        let html = r#"
        <html><body>
            <form action="/search" method="GET">
                <input type="text" name="q" />
                <button type="submit">Search</button>
            </form>
        </body></html>
        "#;

        let method = parse_login_form(html, "https://example.com");
        // No password field, so no login form detected. May return OAuth or None.
        if let Some(LoginMethod::Password { .. }) = method {
            panic!("should not detect a password login form without a password field");
        }
    }
}
