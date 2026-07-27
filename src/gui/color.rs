// SPDX-License-Identifier:  GPL-3.0-or-later

//! Simple Color type that's enough for our needs, along with some logic like alpha-blending.

/// Color in the RGBA color space.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct Color {
    // The color is stored internally in ARGB8888 little-endian format (so, BGRA) with
    // pre-multiplied alpha, for the efficiency reasons - since this is what we use the most. The
    // outside code shouldn't worry about that though, this should be considered an implementation
    // detail that is free to change.
    bgra: [u8; 4],
}

/// "Multiply" a color component by an alpha in 0-255 range. If alpha was in 0-1 range (floating
/// point) then it'd be just a multiplication, but with 0-255 range we need a few type casts. Just
/// a trivial helper to avoid converting everything to floating point numbers all the time.
fn alpha_mul(c: u8, a: u8) -> u8 {
    u8::try_from(u32::from(c) * u32::from(a) / 255).unwrap()
}

impl Color {
    /// Construct a `Color` from rgba components, *without premultiplied alpha*.
    pub fn from_rgba(r: u8, g: u8, b: u8, a: u8) -> Self {
        Self {
            bgra: [alpha_mul(b, a), alpha_mul(g, a), alpha_mul(r, a), a],
        }
    }

    /// Construct a `Color` from a little-endian ARGB8888 representation with pre-multiplied alpha.
    pub fn from_argb_le_premul(bgra: [u8; 4]) -> Self {
        Self { bgra }
    }

    /// Get the `Color` as a little-endian ARGB8888 representation with pre-multiplied alpha.
    pub fn as_argb_le_premul(&self) -> &[u8; 4] {
        &self.bgra
    }

    /// Overlay another color on top of this one (src-over)
    pub fn overlay_other(self, fg: Color) -> Color {
        if fg.bgra[3] == 255 {
            return fg;
        }
        Color {
            bgra: [
                fg.bgra[0] + alpha_mul(self.bgra[0], 255 - fg.bgra[3]),
                fg.bgra[1] + alpha_mul(self.bgra[1], 255 - fg.bgra[3]),
                fg.bgra[2] + alpha_mul(self.bgra[2], 255 - fg.bgra[3]),
                u8::try_from(
                    u32::from(fg.bgra[3]) + u32::from(self.bgra[3])
                        - u32::from(alpha_mul(fg.bgra[3], self.bgra[3])),
                )
                .unwrap(),
            ],
        }
    }

    /// Add "more transparency" to the color
    pub fn add_alpha(self, alpha: u8) -> Color {
        Color {
            bgra: [
                alpha_mul(self.bgra[0], alpha),
                alpha_mul(self.bgra[1], alpha),
                alpha_mul(self.bgra[2], alpha),
                alpha_mul(self.bgra[3], alpha),
            ],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    mod from_rgba {
        use super::*;

        // `Color::from_rgba` premultiplies alpha and stores the result in ARGB8888 little-endian
        // (BGRA) order; `as_argb_le_premul` exposes that raw representation for inspection.

        #[test]
        fn test_opaque_preserves_channels_in_bgra_order() {
            // With full alpha, premultiplication is a no-op, so this also verifies the output
            // channel order is [B, G, R, A].
            let got = *Color::from_rgba(1, 2, 3, 0xff).as_argb_le_premul();
            assert_eq!(got, [3, 2, 1, 0xff]);
        }

        #[test]
        fn test_fully_transparent_is_all_zero() {
            // Alpha of 0 premultiplies every color channel down to 0.
            let got = *Color::from_rgba(10, 20, 30, 0).as_argb_le_premul();
            assert_eq!(got, [0, 0, 0, 0]);
        }

        #[test]
        fn test_opaque_primary_colors() {
            assert_eq!(
                *Color::from_rgba(0xff, 0, 0, 0xff).as_argb_le_premul(),
                [0, 0, 0xff, 0xff]
            );
            assert_eq!(
                *Color::from_rgba(0, 0xff, 0, 0xff).as_argb_le_premul(),
                [0, 0xff, 0, 0xff]
            );
            assert_eq!(
                *Color::from_rgba(0, 0, 0xff, 0xff).as_argb_le_premul(),
                [0xff, 0, 0, 0xff]
            );
        }

        #[test]
        fn test_opaque_black_and_white() {
            assert_eq!(
                *Color::from_rgba(0, 0, 0, 0xff).as_argb_le_premul(),
                [0, 0, 0, 0xff]
            );
            assert_eq!(
                *Color::from_rgba(0xff, 0xff, 0xff, 0xff).as_argb_le_premul(),
                [0xff, 0xff, 0xff, 0xff]
            );
        }

        #[test]
        fn test_half_alpha_premultiplication() {
            // White at 50% alpha: each channel becomes 255 * 128 / 255 == 128.
            assert_eq!(
                *Color::from_rgba(0xff, 0xff, 0xff, 128).as_argb_le_premul(),
                [128, 128, 128, 128]
            );
            // Distinct channels at 50% alpha, checking both the math and the BGRA ordering:
            //   b: 50  * 128 / 255 == 25
            //   g: 100 * 128 / 255 == 50
            //   r: 200 * 128 / 255 == 100
            assert_eq!(
                *Color::from_rgba(200, 100, 50, 128).as_argb_le_premul(),
                [25, 50, 100, 128]
            );
        }

        #[test]
        fn test_from_argb_le_premul_round_trips() {
            // `from_argb_le_premul` stores the raw bytes verbatim, so `as_argb_le_premul` must
            // return exactly what was put in.
            let raw = [10, 20, 30, 200];
            assert_eq!(*Color::from_argb_le_premul(raw).as_argb_le_premul(), raw);
        }
    }

    mod add_alpha {
        use super::*;

        // `Color` stores premultiplied ARGB8888 little-endian (BGRA), so `add_alpha` scales every
        // channel (including the already-premultiplied color channels) by the extra alpha.
        // `Color` derives `PartialEq`/`Eq`, so we compare whole `Color`s directly rather than
        // inspecting the raw byte representation.

        #[test]
        fn test_full_alpha_preserves_everything() {
            // Multiplying by an alpha of 1.0 (255) is a no-op.
            let color = Color::from_argb_le_premul([10, 20, 30, 200]);
            assert_eq!(color.add_alpha(0xff), color);
        }

        #[test]
        fn test_zero_alpha_clears_everything() {
            // Because the stored channels are premultiplied, scaling by an alpha of 0 zeroes them
            // all out, including the color channels.
            let got = Color::from_argb_le_premul([10, 20, 30, 200]).add_alpha(0);
            assert_eq!(got, Color::from_argb_le_premul([0, 0, 0, 0]));
        }

        #[test]
        fn test_half_alpha_on_opaque() {
            // Opaque pixel at 50% additional alpha: alpha 255 * 128 / 255 == 128, and each stored
            // color channel is scaled likewise (e.g. 2 * 128 / 255 == 1, 4 -> 2, 6 -> 3).
            let got = Color::from_argb_le_premul([2, 4, 6, 0xff]).add_alpha(128);
            assert_eq!(got, Color::from_argb_le_premul([1, 2, 3, 128]));
        }

        #[test]
        fn test_two_partial_alphas_multiply() {
            // Every channel at 128 scaled by additional alpha 128: 128 * 128 / 255 == 64
            // (16384 / 255 == 64 after truncation).
            let got = Color::from_argb_le_premul([128, 128, 128, 128]).add_alpha(128);
            assert_eq!(got, Color::from_argb_le_premul([64, 64, 64, 64]));
        }

        #[test]
        fn test_scales_premultiplied_channels() {
            // Start from an opaque rgba color, then add 50% transparency. The premultiplied BGRA of
            // rgba(200, 100, 50, 255) is [50, 100, 200, 255]; scaling by 128 gives:
            //   b: 50  * 128 / 255 == 25
            //   g: 100 * 128 / 255 == 50
            //   r: 200 * 128 / 255 == 100
            //   a: 255 * 128 / 255 == 128
            let got = Color::from_rgba(200, 100, 50, 0xff).add_alpha(128);
            assert_eq!(got, Color::from_argb_le_premul([25, 50, 100, 128]));
        }
    }

    mod overlay_other {
        use super::*;

        // Colors are stored as premultiplied ARGB8888 little-endian, i.e. the byte order is
        // [B, G, R, A]. `bg.overlay_other(fg)` composites `fg` over `bg` (src-over). `Color`
        // derives `PartialEq`/`Eq`, so we assert on whole `Color`s rather than their raw bytes.

        /// Build a `Color` from a raw premultiplied BGRA byte array.
        fn color(bgra: [u8; 4]) -> Color {
            Color::from_argb_le_premul(bgra)
        }

        #[test]
        fn test_opaque_foreground_replaces_background() {
            // A fully opaque foreground takes the fast path and completely overwrites the
            // background, regardless of what the background was.
            let fg = color([10, 20, 30, 0xff]);
            let bg = color([100, 110, 120, 40]);
            assert_eq!(bg.overlay_other(fg), fg);
        }

        #[test]
        fn test_transparent_foreground_leaves_background_unchanged() {
            // A fully transparent foreground is all zeros once premultiplied, so it must not
            // affect the background at all.
            let fg = color([0, 0, 0, 0]);
            let bg = color([100, 110, 120, 200]);
            assert_eq!(bg.overlay_other(fg), bg);
        }

        #[test]
        fn test_overlay_onto_empty_yields_foreground() {
            // Overlaying onto a fully transparent (empty) background reproduces the foreground.
            let fg = color([100, 50, 25, 128]);
            let bg = color([0, 0, 0, 0]);
            assert_eq!(bg.overlay_other(fg), fg);
        }

        #[test]
        fn test_partial_over_opaque_blends_colors() {
            // Half-transparent foreground over an opaque background. Worked example:
            //   1 - a_fg == 255 - 128 == 127
            //   b: 100 + (0   * 127 / 255) == 100
            //   g:   0 + (0   * 127 / 255) == 0
            //   r:   0 + (200 * 127 / 255) == 99   (25400 / 255 == 99 after truncation)
            //   a: 128 + 255 - (128 * 255 / 255) == 255
            let fg = color([100, 0, 0, 128]);
            let bg = color([0, 0, 200, 0xff]);
            assert_eq!(bg.overlay_other(fg), color([100, 0, 99, 0xff]));
        }

        #[test]
        fn test_partial_over_partial_accumulates_alpha() {
            // Two half-transparent pixels combine following a_out = a_fg + a_bg * (1 - a_fg):
            //   a: 128 + 128 - (128 * 128 / 255) == 256 - 64 == 192
            let fg = color([0, 0, 0, 128]);
            let bg = color([0, 0, 0, 128]);
            assert_eq!(bg.overlay_other(fg), color([0, 0, 0, 192]));
        }
    }
}
