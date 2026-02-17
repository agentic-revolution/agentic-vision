//! Interpolate feature vectors for unrendered pages from rendered samples.

use crate::map::types::{PageType, FEAT_CONTENT_FRESHNESS, FEAT_PAGE_TYPE_CONFIDENCE, FEATURE_DIM};

/// Interpolate a feature vector for an unrendered page.
///
/// Averages the feature vectors of rendered pages with the same PageType.
/// Sets confidence to 0.5 and freshness to 0.0 to indicate estimation.
pub fn interpolate_features(
    page_type: PageType,
    samples: &[&[f32; FEATURE_DIM]],
) -> [f32; FEATURE_DIM] {
    if samples.is_empty() {
        // No samples: return a minimal feature vector
        let mut feats = [0.0f32; FEATURE_DIM];
        feats[crate::map::types::FEAT_PAGE_TYPE] = (page_type as u8) as f32 / 31.0;
        feats[FEAT_PAGE_TYPE_CONFIDENCE] = 0.3;
        feats[FEAT_CONTENT_FRESHNESS] = 0.0;
        return feats;
    }

    let mut result = [0.0f32; FEATURE_DIM];
    let n = samples.len() as f32;

    // Average all sample dimensions
    for sample in samples {
        for (i, &val) in sample.iter().enumerate() {
            result[i] += val;
        }
    }
    for val in &mut result {
        *val /= n;
    }

    // Override confidence and freshness
    result[FEAT_PAGE_TYPE_CONFIDENCE] = 0.5;
    result[FEAT_CONTENT_FRESHNESS] = 0.0;

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_interpolate_from_samples() {
        let mut s1 = [0.0f32; FEATURE_DIM];
        s1[0] = 1.0;
        s1[10] = 0.5;

        let mut s2 = [0.0f32; FEATURE_DIM];
        s2[0] = 1.0;
        s2[10] = 0.3;

        let mut s3 = [0.0f32; FEATURE_DIM];
        s3[0] = 1.0;
        s3[10] = 0.8;

        let samples: Vec<&[f32; FEATURE_DIM]> = vec![&s1, &s2, &s3];
        let result = interpolate_features(PageType::Article, &samples);

        // Average of dim 0: (1.0+1.0+1.0)/3 = 1.0
        assert!((result[0] - 1.0).abs() < 0.001);
        // Average of dim 10: (0.5+0.3+0.8)/3 ≈ 0.533
        assert!((result[10] - 0.533).abs() < 0.01);
        // Confidence should be 0.5
        assert_eq!(result[FEAT_PAGE_TYPE_CONFIDENCE], 0.5);
        // Freshness should be 0.0
        assert_eq!(result[FEAT_CONTENT_FRESHNESS], 0.0);
    }

    #[test]
    fn test_interpolate_no_samples() {
        let result = interpolate_features(PageType::ProductDetail, &[]);
        assert_eq!(result[FEAT_PAGE_TYPE_CONFIDENCE], 0.3);
        assert_eq!(result[FEAT_CONTENT_FRESHNESS], 0.0);
    }
}
