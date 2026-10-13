//! Configuration options for html-squish.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SquishConfig {
    /// Collapses redundant whitespace sequences into a single space or eliminates it between tags.
    /// Default: true.
    pub collapse_whitespace: bool,

    /// Aggressively removes all whitespace between tags (`> <` becomes `><`).
    /// Default: true.
    pub aggressive_intertag_whitespace: bool,

    /// Strips HTML comments (`<!-- ... -->`).
    /// Comments starting with `<!--!` (preserve license / banners) are retained if true.
    /// Default: true.
    pub strip_comments: bool,

    /// Strips comments starting with `<!--!` as well.
    /// Default: false.
    pub strip_bang_comments: bool,

    /// Minifies embedded `<script type="application/ld+json">` and `<script type="application/json">` blocks.
    /// Default: true.
    pub minify_json_ld: bool,

    /// Minifies embedded standard `<script>` JavaScript blocks (stripping comments and collapsing whitespace).
    /// Default: true.
    pub minify_js: bool,

    /// Minifies embedded `<style>` CSS blocks (stripping comments and collapsing whitespace).
    /// Default: true.
    pub minify_css: bool,

    /// Preserves verbatim whitespace inside `<pre>` and `<textarea>` elements.
    /// Default: true.
    pub preserve_pre_and_textarea: bool,

    /// Removes unneeded whitespace around attribute `=` signs (`class = "foo"` -> `class="foo"`).
    /// Default: true.
    pub normalize_attributes: bool,
}

impl Default for SquishConfig {
    #[inline]
    fn default() -> Self {
        Self {
            collapse_whitespace: true,
            aggressive_intertag_whitespace: true,
            strip_comments: true,
            strip_bang_comments: false,
            minify_json_ld: true,
            minify_js: true,
            minify_css: true,
            preserve_pre_and_textarea: true,
            normalize_attributes: true,
        }
    }
}

impl SquishConfig {
    /// Ultra-aggressive minification preset for maximum compression.
    #[inline]
    pub const fn aggressive() -> Self {
        Self {
            collapse_whitespace: true,
            aggressive_intertag_whitespace: true,
            strip_comments: true,
            strip_bang_comments: true,
            minify_json_ld: true,
            minify_js: true,
            minify_css: true,
            preserve_pre_and_textarea: true,
            normalize_attributes: true,
        }
    }

    /// Conservative minification preset that preserves inter-tag spaces (`> <`).
    #[inline]
    pub const fn conservative() -> Self {
        Self {
            collapse_whitespace: true,
            aggressive_intertag_whitespace: false,
            strip_comments: true,
            strip_bang_comments: false,
            minify_json_ld: true,
            minify_js: true,
            minify_css: true,
            preserve_pre_and_textarea: true,
            normalize_attributes: true,
        }
    }
}
