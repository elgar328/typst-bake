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
/// Each variant maps 1:1 to a single typst PDF standard, and [`PdfConfig`] applies one
/// standard.
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

/// A PDF creation timestamp: an instant, written with a UTC offset.
///
/// - [`PdfTimestamp::now`]: the current time, with the local UTC offset.
/// - [`PdfTimestamp::from_unix`]: a given instant, in UTC.
/// - [`PdfTimestamp::with_offset_minutes`]: the same instant, written at another offset.
///
/// The value is applied only when the template's document date is `auto`; a
/// `#set document(date: ..)` in the template takes precedence, which is also the simplest way
/// to fix a date. A timestamp that a PDF date cannot represent (a year outside 0000–9999 after
/// the offset, or an unsupported offset) is reported as an error when the PDF is generated.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PdfTimestamp {
    /// Seconds since 1970-01-01T00:00:00Z.
    unix_secs: i64,
    /// Minutes offset from UTC; 0 is written as UTC.
    offset_minutes: i32,
}

// `now` reads the clock. wasm32-unknown-unknown has none unless `js-clock` reads it from
// JavaScript (chrono can do that on wasm32 only); there `SystemTime::now` panics.
#[cfg(not(all(
    target_family = "wasm",
    target_os = "unknown",
    not(all(target_arch = "wasm32", feature = "js-clock"))
)))]
// The condition reads poorly as a docs.rs badge; `now` explains it instead.
#[cfg_attr(docsrs, doc(auto_cfg = false))]
impl PdfTimestamp {
    /// The current time with the local UTC offset, as the Typst CLI writes it.
    ///
    /// If the clock predates the Unix epoch, the time saturates to `1970-01-01T00:00:00Z`.
    ///
    /// On `wasm32-unknown-unknown` this needs the `js-clock` feature (on by default), which
    /// reads the time from JavaScript. Other WebAssembly targets without an OS have no clock
    /// and no `now`; use [`PdfTimestamp::from_unix`] there.
    pub fn now() -> Self {
        let unix_secs = Self::now_unix_secs();
        let offset_minutes = chrono::DateTime::from_timestamp(unix_secs, 0)
            .map(|t| t.with_timezone(&chrono::Local).offset().local_minus_utc() / 60)
            .unwrap_or(0);
        Self {
            unix_secs,
            offset_minutes,
        }
    }

    /// Read the current Unix time in seconds, saturating to 0 if the clock predates the
    /// epoch.
    fn now_unix_secs() -> i64 {
        if cfg!(all(target_family = "wasm", target_os = "unknown")) {
            // JavaScript `Date`, through chrono's `wasmbind` (enabled by `js-clock`).
            chrono::Utc::now().timestamp().max(0)
        } else {
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_secs() as i64)
                .unwrap_or(0)
        }
    }
}

/// The largest UTC offset a PDF date can carry: 23 hours 59 minutes.
const MAX_OFFSET_MINUTES: i32 = 23 * 60 + 59;

impl PdfTimestamp {
    /// A Unix timestamp (seconds since 1970-01-01T00:00:00Z), written in UTC.
    ///
    /// Works on every target, including those without a clock.
    pub fn from_unix(unix_secs: i64) -> Self {
        Self {
            unix_secs,
            offset_minutes: 0,
        }
    }

    /// The same instant, written as wall-clock time at the given UTC offset in minutes.
    ///
    /// For example, `PdfTimestamp::now().with_offset_minutes(540)` writes the current time
    /// in UTC+09:00 regardless of the machine's time zone. An offset of 0 writes UTC.
    pub fn with_offset_minutes(self, offset_minutes: i32) -> Self {
        Self {
            offset_minutes,
            ..self
        }
    }

    /// The wall-clock date and time at the offset, checked against what a PDF date holds.
    fn civil(self) -> std::result::Result<(i32, u8, u8, u8, u8, u8), String> {
        let offset = self.offset_minutes;
        // `typst_pdf::Timestamp::new_local` loses the sign of offsets between -1 and -59
        // minutes (the hour part is 0), so they would be written as positive.
        if !(-MAX_OFFSET_MINUTES..=MAX_OFFSET_MINUTES).contains(&offset)
            || (-59..=-1).contains(&offset)
        {
            return Err(format!(
                "timestamp offset of {offset} minutes is not supported \
                 (must be within ±23:59 and not between -00:59 and -00:01)"
            ));
        }
        // typst-pdf drops the date of a negative year, and krilla clamps years past 9999,
        // so check the range here rather than rely on `Datetime`.
        let out_of_range = || "timestamp year is outside 0000–9999".to_string();
        let local = self
            .unix_secs
            .checked_add(i64::from(offset) * 60)
            .ok_or_else(out_of_range)?;
        let (year, month, day, hour, minute, second) = civil_from_unix(local);
        if !(0..=9999).contains(&year) {
            return Err(out_of_range());
        }
        Ok((year as i32, month, day, hour, minute, second))
    }

    /// Convert to a typst timestamp, or describe why PDF cannot represent it.
    fn to_typst(self) -> std::result::Result<typst_pdf::Timestamp, String> {
        let (year, month, day, hour, minute, second) = self.civil()?;
        let datetime =
            typst::foundations::Datetime::from_ymd_hms(year, month, day, hour, minute, second)
                .ok_or("invalid timestamp")?;
        if self.offset_minutes == 0 {
            Ok(typst_pdf::Timestamp::new_utc(datetime))
        } else {
            typst_pdf::Timestamp::new_local(datetime, self.offset_minutes)
                .ok_or_else(|| "invalid timestamp offset".to_string())
        }
    }
}

/// Convert Unix time (seconds) to `(year, month, day, hour, minute, second)` in UTC.
///
/// Uses Howard Hinnant's `civil_from_days` algorithm. Euclidean division/remainder are
/// used so pre-epoch (negative) inputs are handled correctly.
///
/// The year is returned as `i64`: far-off timestamps do not fit an `i32`.
fn civil_from_unix(secs: i64) -> (i64, u8, u8, u8, u8, u8) {
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
    let year = y + if month <= 2 { 1 } else { 0 };

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
///     timestamp: Some(PdfTimestamp::now()),
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
    /// The producing application, written to the PDF `/Creator` metadata field.
    ///
    /// `None` (the default) keeps typst's own `"Typst x.y.z"` string. Set this to
    /// identify your own application instead.
    pub creator: Option<String>,
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
            creator: None,
        }
    }
}

/// Render a typst `HintedString` as a single message.
///
/// `HintedString` dropped its `Display` impl in typst 0.15, so join the message with
/// its hints by hand.
fn hinted_to_string(error: &typst::diag::HintedString) -> String {
    let mut out = error.message().to_string();
    for hint in error.hints() {
        out.push_str("; hint: ");
        out.push_str(hint);
    }
    out
}

impl PdfConfig {
    /// Convert to typst PDF options.
    ///
    /// `page_ranges` is always `None` here; page selection is applied by the renderer.
    pub(crate) fn to_typst(&self) -> Result<typst_pdf::PdfOptions> {
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
            .map_err(|e| Error::InvalidPdfConfig(hinted_to_string(&e)))?;

        let timestamp = self
            .timestamp
            .map(PdfTimestamp::to_typst)
            .transpose()
            .map_err(Error::InvalidPdfConfig)?;

        Ok(typst_pdf::PdfOptions {
            ident: self.ident.clone().map(Smart::Custom).unwrap_or(Smart::Auto),
            creator: self
                .creator
                .clone()
                .map(|c| Smart::Custom(Some(c)))
                .unwrap_or(Smart::Auto),
            pretty: false,
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
        // `Auto` leaves typst's own "Typst x.y.z" creator string in place.
        assert!(matches!(opts.creator, typst::foundations::Smart::Auto));
        assert!(opts.timestamp.is_none());
        assert!(opts.page_ranges.is_none());
    }

    #[test]
    fn custom_creator_is_forwarded() {
        let cfg = PdfConfig {
            creator: Some("My Invoice App 1.0".into()),
            ..Default::default()
        };
        let opts = cfg.to_typst().unwrap();
        match opts.creator {
            typst::foundations::Smart::Custom(Some(c)) => assert_eq!(c, "My Invoice App 1.0"),
            other => panic!("expected a custom creator, got {other:?}"),
        }
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

    /// 2026-06-06T12:00:00Z.
    const JUNE_6_NOON: i64 = 1_780_747_200;

    #[test]
    fn from_unix_is_utc() {
        assert_eq!(
            PdfTimestamp::from_unix(0).civil(),
            Ok((1970, 1, 1, 0, 0, 0))
        );
        assert_eq!(
            PdfTimestamp::from_unix(JUNE_6_NOON).civil(),
            Ok((2026, 6, 6, 12, 0, 0))
        );
    }

    #[test]
    fn years_outside_0000_to_9999_are_rejected() {
        let civil = |secs| PdfTimestamp::from_unix(secs).civil();
        // 0000-01-01T00:00:00Z and 9999-12-31T23:59:59Z are the bounds.
        assert_eq!(civil(-62_167_219_200), Ok((0, 1, 1, 0, 0, 0)));
        assert!(civil(-62_167_219_201).is_err());
        assert_eq!(civil(253_402_300_799), Ok((9999, 12, 31, 23, 59, 59)));
        assert!(civil(253_402_300_800).is_err());
        for bound in [-62_167_219_200, 253_402_300_799] {
            assert!(PdfTimestamp::from_unix(bound).to_typst().is_ok(), "{bound}");
        }
        assert!(civil(i64::MAX).is_err());
        assert!(civil(i64::MIN).is_err());
        // The year of this instant is 2026 + 2^28 * 400, which wraps to exactly 2026 as an i32.
        assert!(civil(3_388_401_921_816_792_000).is_err());
        // The bound applies to the wall-clock year: one hour west of 0000-01-01T00:00Z is
        // year -1.
        let start = PdfTimestamp::from_unix(-62_167_219_200);
        assert!(start.with_offset_minutes(60).civil().is_ok());
        assert!(start.with_offset_minutes(-60).civil().is_err());
    }

    #[test]
    fn with_offset_minutes_shifts_the_wall_clock() {
        let noon = PdfTimestamp::from_unix(JUNE_6_NOON);
        assert_eq!(
            noon.with_offset_minutes(330).civil(),
            Ok((2026, 6, 6, 17, 30, 0))
        );
        assert_eq!(
            noon.with_offset_minutes(-210).civil(),
            Ok((2026, 6, 6, 8, 30, 0))
        );
        // Crosses into the next day.
        assert_eq!(
            noon.with_offset_minutes(13 * 60).civil(),
            Ok((2026, 6, 7, 1, 0, 0))
        );
        // Offset 0 is UTC.
        assert_eq!(noon.with_offset_minutes(0), noon);
    }

    #[test]
    fn unsupported_offsets_are_rejected() {
        let noon = PdfTimestamp::from_unix(JUNE_6_NOON);
        for offset in [24 * 60, -24 * 60, -30, -1, -59] {
            assert!(
                noon.with_offset_minutes(offset).civil().is_err(),
                "{offset}"
            );
        }
        for offset in [23 * 60 + 59, -(23 * 60 + 59), -60, 1, 59] {
            assert!(
                noon.with_offset_minutes(offset).to_typst().is_ok(),
                "{offset}"
            );
        }
    }

    #[test]
    fn invalid_timestamps_fail_the_config() {
        let cfg = PdfConfig {
            timestamp: Some(PdfTimestamp::from_unix(i64::MAX)),
            ..Default::default()
        };
        assert!(matches!(cfg.to_typst(), Err(Error::InvalidPdfConfig(_))));
    }

    #[test]
    fn now_converts() {
        assert!(PdfTimestamp::now().to_typst().is_ok());
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
