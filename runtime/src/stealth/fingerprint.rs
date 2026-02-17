//! Browser fingerprint patching — hide automation signals.

/// JavaScript to inject that patches navigator.webdriver and chrome.runtime.
pub const STEALTH_SCRIPT: &str = r#"
(() => {
    // Hide webdriver flag
    Object.defineProperty(navigator, 'webdriver', {
        get: () => false,
        configurable: true,
    });

    // Patch chrome.runtime to look like a real browser
    if (!window.chrome) {
        window.chrome = {};
    }
    if (!window.chrome.runtime) {
        window.chrome.runtime = {
            connect: function() {},
            sendMessage: function() {},
        };
    }

    // Override permissions query to hide "notifications" prompt
    const originalQuery = window.navigator.permissions.query;
    window.navigator.permissions.query = (parameters) =>
        parameters.name === 'notifications'
            ? Promise.resolve({ state: Notification.permission })
            : originalQuery(parameters);

    // Patch plugins to appear non-empty
    Object.defineProperty(navigator, 'plugins', {
        get: () => [1, 2, 3, 4, 5],
        configurable: true,
    });

    // Patch languages
    Object.defineProperty(navigator, 'languages', {
        get: () => ['en-US', 'en'],
        configurable: true,
    });
})();
"#;

/// Get the stealth injection script.
pub fn stealth_script() -> &'static str {
    STEALTH_SCRIPT
}
