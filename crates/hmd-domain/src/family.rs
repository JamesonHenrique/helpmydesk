// SPDX-License-Identifier: GPL-3.0-or-later
//! Problem families covered by the MVP (Plan v1 III.11).

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ProblemFamily {
    Connectivity,
    Performance,
    Storage,
    AudioVideo,
    AppError,
}

impl ProblemFamily {
    pub const ALL: [ProblemFamily; 5] = [
        ProblemFamily::Connectivity,
        ProblemFamily::Performance,
        ProblemFamily::Storage,
        ProblemFamily::AudioVideo,
        ProblemFamily::AppError,
    ];

    /// Stable identifier used in the catalog and in AI task enums.
    pub fn code(self) -> &'static str {
        match self {
            ProblemFamily::Connectivity => "connectivity",
            ProblemFamily::Performance => "performance",
            ProblemFamily::Storage => "storage",
            ProblemFamily::AudioVideo => "audio_video",
            ProblemFamily::AppError => "app_error",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn codes_are_unique() {
        let codes: HashSet<_> = ProblemFamily::ALL.iter().map(|f| f.code()).collect();
        assert_eq!(codes.len(), ProblemFamily::ALL.len());
    }
}
