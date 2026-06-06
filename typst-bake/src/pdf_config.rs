//! Builder-phase PDF export options.
//!
//! [`PdfConfig`] is passed to [`Document::with_pdf_config`](crate::Document::with_pdf_config)
//! to control PDF-only export settings (tagging, conformance standard, document
//! identifier, creation timestamp). These options affect the PDF export stage only;
//! SVG/PNG output ignores them.
//!
//! All typst-pdf coupling is isolated to the private conversion functions in this
//! module, so a typst version bump only needs to be checked here.

use crate::error::{Error, Result};

/// A PDF conformance standard to enforce on export.
///
/// Each variant maps 1:1 to a single typst PDF standard. typst 0.14 enforces at most
/// one substandard at a time, so PDF/A and PDF/UA cannot be combined.
///
/// The accessible PDF/A levels (`A1a`, `A2a`, `A3a`) and `Ua1` require a tagged PDF;
/// combining them with `tagged: false` or with page selection returns
/// [`Error::InvalidPdfConfig`]. The basic/unicode levels and `A4*` do not require tagging.
///
/// Note: any PDF/A standard requires a document date. Provide one via
/// [`PdfConfig::timestamp`] or `#set document(date: ..)` in the template; otherwise
/// export fails with a "missing document date" error.
//
// Note: this enum is intentionally NOT `#[non_exhaustive]` so callers can match it
// without a wildcard arm. The trade-off is that adding a variant later (if typst gains
// a new standard) is a breaking change, handled by the version policy at that time.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum PdfStandard {
    /// PDF 1.4.
    V1_4,
    /// PDF 1.5.
    V1_5,
    /// PDF 1.6.
    V1_6,
    /// PDF 1.7. This is the default.
    #[default]
    V1_7,
    /// PDF 2.0.
    V2_0,
    /// PDF/A-1b (basic conformance).
    A1b,
    /// PDF/A-1a (accessible conformance; requires tagging).
    A1a,
    /// PDF/A-2b (basic conformance).
    A2b,
    /// PDF/A-2u (unicode conformance).
    A2u,
    /// PDF/A-2a (accessible conformance; requires tagging).
    A2a,
    /// PDF/A-3b (basic conformance).
    A3b,
    /// PDF/A-3u (unicode conformance).
    A3u,
    /// PDF/A-3a (accessible conformance; requires tagging).
    A3a,
    /// PDF/A-4.
    A4,
    /// PDF/A-4f.
    A4f,
    /// PDF/A-4e.
    A4e,
    /// PDF/UA-1 (accessibility; requires tagging).
    Ua1,
}

impl PdfStandard {
    // `typst_pdf::PdfStandard` is `#[non_exhaustive]` at the enum level, but constructing
    // its existing variants from outside the crate is allowed (only exhaustive matching is
    // restricted). So this 1:1 construction mapping compiles.
    fn to_typst(self) -> typst_pdf::PdfStandard {
        use typst_pdf::PdfStandard as T;
        match self {
            PdfStandard::V1_4 => T::V_1_4,
            PdfStandard::V1_5 => T::V_1_5,
            PdfStandard::V1_6 => T::V_1_6,
            PdfStandard::V1_7 => T::V_1_7,
            PdfStandard::V2_0 => T::V_2_0,
            PdfStandard::A1b => T::A_1b,
            PdfStandard::A1a => T::A_1a,
            PdfStandard::A2b => T::A_2b,
            PdfStandard::A2u => T::A_2u,
            PdfStandard::A2a => T::A_2a,
            PdfStandard::A3b => T::A_3b,
            PdfStandard::A3u => T::A_3u,
            PdfStandard::A3a => T::A_3a,
            PdfStandard::A4 => T::A_4,
            PdfStandard::A4f => T::A_4f,
            PdfStandard::A4e => T::A_4e,
            PdfStandard::Ua1 => T::Ua_1,
        }
    }

    /// Whether this standard mandates a tagged PDF (structure tree).
    ///
    /// Mirrors krilla's `Validator::requires_tagging`: the accessible PDF/A levels
    /// (`A1a`, `A2a`, `A3a`) and `Ua1` require tagging.
    pub(crate) fn requires_tagging(self) -> bool {
        matches!(
            self,
            PdfStandard::A1a | PdfStandard::A2a | PdfStandard::A3a | PdfStandard::Ua1
        )
    }
}

/// A PDF creation timestamp.
///
/// Stores plain calendar fields (no external date dependency). The value is applied
/// only when the template's document date is `auto`; a `#set document(date: ..)` in the
/// template takes precedence.
///
/// For the common case of "now in UTC", use [`PdfTimestamp::now_utc`]. To attach a
/// timezone offset, supply it explicitly with [`PdfTimestamp::now_local`] or
/// [`PdfTimestamp::local`] — local timezone auto-detection is not supported.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PdfTimestamp {
    year: i32,
    month: u8,
    day: u8,
    hour: u8,
    minute: u8,
    second: u8,
    /// Minutes offset from UTC; `None` means UTC.
    offset_minutes: Option<i32>,
}

/// Valid whole-minute UTC offset range, matching `typst_pdf::Timestamp::new_local`.
fn valid_offset(minutes: i32) -> bool {
    (-(23 * 60 + 59)..=(23 * 60 + 59)).contains(&minutes)
}

impl PdfTimestamp {
    /// The current time in UTC.
    ///
    /// This is the common case. It never panics: if the system clock predates the Unix
    /// epoch, it saturates to `1970-01-01T00:00:00Z`.
    pub fn now_utc() -> Self {
        let (year, month, day, hour, minute, second) = civil_from_unix(now_unix_secs());
        Self {
            year,
            month,
            day,
            hour,
            minute,
            second,
            offset_minutes: None,
        }
    }

    /// The current time expressed as wall-clock time at the given UTC offset (in minutes).
    ///
    /// For example, `now_local(540)` yields the current time in UTC+09:00 (KST).
    /// Returns `None` if the offset is outside ±(23h, 59m).
    pub fn now_local(offset_minutes: i32) -> Option<Self> {
        if !valid_offset(offset_minutes) {
            return None;
        }
        let (year, month, day, hour, minute, second) =
            civil_from_unix(now_unix_secs() + offset_minutes as i64 * 60);
        Some(Self {
            year,
            month,
            day,
            hour,
            minute,
            second,
            offset_minutes: Some(offset_minutes),
        })
    }

    /// A specific UTC date and time.
    ///
    /// Returns `None` if the date or time is invalid (e.g. month 13).
    pub fn utc(year: i32, month: u8, day: u8, hour: u8, minute: u8, second: u8) -> Option<Self> {
        // Validate via typst's calendar logic.
        typst::foundations::Datetime::from_ymd_hms(year, month, day, hour, minute, second)?;
        Some(Self {
            year,
            month,
            day,
            hour,
            minute,
            second,
            offset_minutes: None,
        })
    }

    /// A specific date and time at the given UTC offset (in minutes).
    ///
    /// Returns `None` if the date/time is invalid or the offset is outside ±(23h, 59m).
    pub fn local(
        year: i32,
        month: u8,
        day: u8,
        hour: u8,
        minute: u8,
        second: u8,
        offset_minutes: i32,
    ) -> Option<Self> {
        typst::foundations::Datetime::from_ymd_hms(year, month, day, hour, minute, second)?;
        if !valid_offset(offset_minutes) {
            return None;
        }
        Some(Self {
            year,
            month,
            day,
            hour,
            minute,
            second,
            offset_minutes: Some(offset_minutes),
        })
    }

    /// Convert to a typst timestamp. Fields were validated at construction, so this
    /// returns `Some` on the normal path; a `None` is surfaced as an error upstream
    /// rather than panicking.
    fn to_typst(self) -> Option<typst_pdf::Timestamp> {
        let datetime = typst::foundations::Datetime::from_ymd_hms(
            self.year,
            self.month,
            self.day,
            self.hour,
            self.minute,
            self.second,
        )?;
        match self.offset_minutes {
            None => Some(typst_pdf::Timestamp::new_utc(datetime)),
            Some(offset) => typst_pdf::Timestamp::new_local(datetime, offset),
        }
    }
}

/// Read the current Unix time in seconds, saturating to 0 if the clock predates the epoch.
fn now_unix_secs() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

/// Convert Unix time (seconds) to `(year, month, day, hour, minute, second)` in UTC.
///
/// Uses Howard Hinnant's `civil_from_days` algorithm. Euclidean division/remainder are
/// used so pre-epoch (negative) inputs are handled correctly.
fn civil_from_unix(secs: i64) -> (i32, u8, u8, u8, u8, u8) {
    let days = secs.div_euclid(86_400);
    let rem = secs.rem_euclid(86_400);
    let hour = (rem / 3_600) as u8;
    let minute = ((rem % 3_600) / 60) as u8;
    let second = (rem % 60) as u8;

    // Howard Hinnant's civil_from_days (days are relative to 1970-01-01).
    let z = days + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = z - era * 146_097; // [0, 146096]
    let yoe = (doe - doe / 1_460 + doe / 36_524 - doe / 146_096) / 365; // [0, 399]
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100); // [0, 365]
    let mp = (5 * doy + 2) / 153; // [0, 11]
    let day = (doy - (153 * mp + 2) / 5 + 1) as u8; // [1, 31]
    let month = (if mp < 10 { mp + 3 } else { mp - 9 }) as u8; // [1, 12]
    let year = (y + if month <= 2 { 1 } else { 0 }) as i32;

    (year, month, day, hour, minute, second)
}

/// PDF export configuration for [`Document::with_pdf_config`](crate::Document::with_pdf_config).
///
/// Construct with struct-update syntax over [`Default`]:
/// ```
/// use typst_bake::{PdfConfig, PdfStandard, PdfTimestamp};
///
/// let config = PdfConfig {
///     tagged: false, // smaller PDF; bookmarks/outline are preserved
///     standard: PdfStandard::A2b,
///     ident: Some("invoice-2026-001".into()),
///     timestamp: Some(PdfTimestamp::now_utc()),
///     ..Default::default()
/// };
/// assert!(!config.tagged);
/// ```
///
/// [`PdfConfig::default()`] maps exactly to typst's default PDF options (tagged PDF,
/// PDF 1.7, auto identifier, no explicit timestamp), so leaving it untouched does not
/// change output.
#[derive(Clone, Debug)]
pub struct PdfConfig {
    /// PDF conformance standard. Defaults to [`PdfStandard::V1_7`].
    pub standard: PdfStandard,
    /// Whether to emit a tagged PDF (accessibility structure tree).
    ///
    /// Defaults to `true`, matching typst. Set to `false` to reduce file size; the
    /// document outline (bookmarks) is independent of tagging and is preserved.
    /// Standards that require tagging (the accessible levels `A1a`/`A2a`/`A3a` and `Ua1`)
    /// reject `false`.
    pub tagged: bool,
    /// A stable document identifier. `None` (the default) lets typst derive one
    /// automatically. Must not be empty.
    pub ident: Option<String>,
    /// The document creation timestamp. Applied only when the template's document date
    /// is `auto`. Required for any PDF/A standard (which mandates a document date) unless
    /// the template sets the date itself.
    pub timestamp: Option<PdfTimestamp>,
}

impl Default for PdfConfig {
    fn default() -> Self {
        // `tagged: true` mirrors typst's default; using `bool::default()` (false) would
        // silently change output for users who don't set a config.
        Self {
            standard: PdfStandard::default(),
            tagged: true,
            ident: None,
            timestamp: None,
        }
    }
}

impl PdfConfig {
    /// Convert to typst PDF options. Borrows `self` for the `ident` string.
    ///
    /// `page_ranges` is always `None` here; page selection is applied by the renderer.
    pub(crate) fn to_typst(&self) -> Result<typst_pdf::PdfOptions<'_>> {
        use typst::foundations::Smart;

        // An accessible standard requires tagging; `tagged: false` would be rejected by
        // typst-pdf anyway, so fail early with a clear message.
        if self.standard.requires_tagging() && !self.tagged {
            return Err(Error::InvalidPdfConfig(format!(
                "{:?} requires tagged PDF; remove `tagged: false`",
                self.standard
            )));
        }
        // An empty identifier would undermine the PDF/A stable-ID guarantee.
        if matches!(&self.ident, Some(s) if s.is_empty()) {
            return Err(Error::InvalidPdfConfig(
                "ident must not be empty; use None for an automatic identifier".into(),
            ));
        }

        let standards = typst_pdf::PdfStandards::new(&[self.standard.to_typst()])
            .map_err(|e| Error::InvalidPdfConfig(e.to_string()))?;

        let timestamp = match self.timestamp {
            Some(ts) => Some(
                ts.to_typst()
                    .ok_or_else(|| Error::InvalidPdfConfig("invalid timestamp".into()))?,
            ),
            None => None,
        };

        Ok(typst_pdf::PdfOptions {
            ident: self
                .ident
                .as_deref()
                .map(Smart::Custom)
                .unwrap_or(Smart::Auto),
            timestamp,
            page_ranges: None,
            standards,
            tagged: self.tagged,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_config_matches_typst_defaults() {
        let cfg = PdfConfig::default();
        let opts = cfg.to_typst().unwrap();
        assert!(opts.tagged);
        assert!(matches!(opts.ident, typst::foundations::Smart::Auto));
        assert!(opts.timestamp.is_none());
        assert!(opts.page_ranges.is_none());
    }

    #[test]
    fn tagged_false_is_ok_for_basic_standards() {
        let cfg = PdfConfig {
            tagged: false,
            standard: PdfStandard::A2b,
            ..Default::default()
        };
        assert!(cfg.to_typst().is_ok());
    }

    #[test]
    fn accessible_standard_rejects_untagged() {
        for standard in [
            PdfStandard::A1a,
            PdfStandard::A2a,
            PdfStandard::A3a,
            PdfStandard::Ua1,
        ] {
            let cfg = PdfConfig {
                tagged: false,
                standard,
                ..Default::default()
            };
            assert!(matches!(cfg.to_typst(), Err(Error::InvalidPdfConfig(_))));
        }
    }

    #[test]
    fn empty_ident_is_rejected() {
        let cfg = PdfConfig {
            ident: Some(String::new()),
            ..Default::default()
        };
        assert!(matches!(cfg.to_typst(), Err(Error::InvalidPdfConfig(_))));
    }

    #[test]
    fn representative_standards_convert() {
        for standard in [PdfStandard::A2b, PdfStandard::V2_0, PdfStandard::A4] {
            let cfg = PdfConfig {
                standard,
                ..Default::default()
            };
            assert!(cfg.to_typst().is_ok());
        }
    }

    #[test]
    fn timestamp_constructors() {
        assert!(PdfTimestamp::utc(2026, 6, 6, 12, 0, 0).is_some());
        assert!(PdfTimestamp::utc(2026, 13, 1, 0, 0, 0).is_none());
        assert!(PdfTimestamp::local(2026, 6, 6, 12, 0, 0, 540).is_some());
        assert!(PdfTimestamp::now_local(99 * 60).is_none());
        // now_utc is infallible and converts cleanly.
        assert!(PdfTimestamp::now_utc().to_typst().is_some());
    }

    #[test]
    fn now_local_offset_shifts_wall_clock() {
        // At the same instant, +60min wall clock is one hour ahead of UTC (modulo day wrap).
        let utc = PdfTimestamp::now_utc();
        let local = PdfTimestamp::now_local(60).unwrap();
        let utc_minutes = utc.hour as i32 * 60 + utc.minute as i32;
        let local_minutes = local.hour as i32 * 60 + local.minute as i32;
        let diff = (local_minutes - utc_minutes).rem_euclid(24 * 60);
        // Allow a 1-minute slack for the (tiny) time between the two clock reads.
        assert!(diff == 60 || diff == 59 || diff == 61, "diff was {diff}");
    }

    #[test]
    fn civil_from_unix_known_values() {
        assert_eq!(civil_from_unix(0), (1970, 1, 1, 0, 0, 0));
        // 2026-06-06T12:00:00Z
        assert_eq!(civil_from_unix(1_780_747_200), (2026, 6, 6, 12, 0, 0));
        // Leap day 2000-02-29 (year-2000 is a leap year).
        assert_eq!(civil_from_unix(951_782_400), (2000, 2, 29, 0, 0, 0));
        // 2100-03-01 (year-2100 is NOT a leap year, so Feb has 28 days).
        assert_eq!(civil_from_unix(4_107_542_400), (2100, 3, 1, 0, 0, 0));
        // Pre-epoch: 1969-12-31T23:59:59Z.
        assert_eq!(civil_from_unix(-1), (1969, 12, 31, 23, 59, 59));
    }
}
