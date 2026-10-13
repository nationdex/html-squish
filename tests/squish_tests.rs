use html_squish::{squish, squish_in_place, squish_in_place_str, squish_slice, SquishConfig};

#[test]
fn test_basic_html_collapse() {
    let input = "  <div>  \n  <p>  Hello    World  </p>  \n  </div>  ";
    let res = squish(input);
    assert_eq!(res, "<div><p>Hello World</p></div>");
}

#[test]
fn test_comments_stripped() {
    let input = "<div><!-- This is a comment --><span>Content</span><!-- Another comment --></div>";
    let res = squish(input);
    assert_eq!(res, "<div><span>Content</span></div>");
}

#[test]
fn test_bang_comment_preserved_by_default() {
    let input = "<!--! License: MIT --><div>Content</div>";
    let res = squish(input);
    assert_eq!(res, "<!--! License: MIT --><div>Content</div>");

    let aggressive_res = html_squish::squish_with_config(input, &SquishConfig::aggressive());
    assert_eq!(aggressive_res, "<div>Content</div>");
}

#[test]
fn test_attributes_normalization() {
    let input = r#"<div   class = "btn  active"   id = 'main'   data-v = "123"   >"#;
    let res = squish(input);
    assert_eq!(res, r#"<div class="btn  active" id='main' data-v="123">"#);
}

#[test]
fn test_self_closing_tag() {
    let input = r#"<img   src="pic.png"   alt="Photo"   />"#;
    let res = squish(input);
    assert_eq!(res, r#"<img src="pic.png" alt="Photo"/>"#);
}

#[test]
fn test_json_ld_minification() {
    let input = r#"
        <html>
        <head>
            <script type="application/ld+json">
            {
                "@context": "https://schema.org",
                "@type": "WebSite",
                "name": "MindVote",
                "url": "https://mindvote.com",
                "potentialAction": {
                    "@type": "SearchAction",
                    "target": "https://mindvote.com/search?q={search_term_string}"
                }
            }
            </script>
        </head>
        <body>
            <h1>MindVote</h1>
        </body>
        </html>
    "#;

    let res = squish(input);
    assert!(res.contains(
        r#"<script type="application/ld+json">{"@context":"https://schema.org","@type":"WebSite","name":"MindVote","url":"https://mindvote.com","potentialAction":{"@type":"SearchAction","target":"https://mindvote.com/search?q={search_term_string}"}}</script>"#
    ));
    assert!(res.contains("<body><h1>MindVote</h1></body></html>"));
}

#[test]
fn test_inline_js_minification() {
    let input = r#"
        <script>
            // This is a single line comment
            const x = 42;
            /* Multi
               line comment */
            function calculate( a , b ) {
                return a + b;
            }
        </script>
    "#;

    let res = squish(input);
    assert!(!res.contains("single line comment"));
    assert!(!res.contains("Multi"));
    assert!(res.contains("const x=42;") || res.contains("const x = 42;"));
    assert!(res.contains("function calculate(a,b)") || res.contains("function calculate"));
}

#[test]
fn test_inline_css_minification() {
    let input = r#"
        <style>
            /* Reset styles */
            body {
                margin: 0;
                padding: 0;
                background-color: #fff;
            }
        </style>
    "#;

    let res = squish(input);
    assert!(!res.contains("Reset styles"));
    assert!(res.contains("body{") || res.contains("body {"));
}

#[test]
fn test_pre_and_textarea_preservation() {
    let input = "<pre>   Line 1\n   Line 2\n\tTabbed\n</pre><textarea>  foo   \n  bar </textarea>";
    let res = squish(input);
    assert_eq!(
        res,
        "<pre>   Line 1\n   Line 2\n\tTabbed\n</pre><textarea>  foo   \n  bar </textarea>"
    );
}

#[test]
fn test_in_place_mutation() {
    let mut data = *b"<div   class=\"test\"  >  <p>  A   B  </p>  </div>";
    let len = squish_in_place(&mut data);
    let result = core::str::from_utf8(&data[..len]).unwrap();
    assert_eq!(result, "<div class=\"test\"><p>A B</p></div>");
}

#[test]
fn test_in_place_str() {
    let mut s = String::from("  <span>  Hello  </span>  <span>  World  </span>  ");
    let minified = squish_in_place_str(&mut s);
    assert_eq!(minified, "<span>Hello</span><span>World</span>");
}

#[test]
fn test_slice_buffer() {
    let input = b"  <div>   Test   </div>  ";
    let mut out = [0u8; 64];
    let len = squish_slice(input, &mut out, &SquishConfig::default()).unwrap();
    assert_eq!(&out[..len], b"<div>Test</div>");
}

#[test]
fn test_utf8_multibyte_preservation() {
    let input = "<div>  🚀 MindVote 🗳️ 投票  —  ¡Éxito!  </div>";
    let res = squish(input);
    assert_eq!(res, "<div>🚀 MindVote 🗳️ 投票 — ¡Éxito!</div>");
}

#[test]
fn test_tag_word_spacing() {
    // Crucial for semantic rendering: <b>bold</b> text must not become <b>bold</b>text
    let input = "<p><b>bold</b> text and <i>italic</i> word</p>";
    let res = squish(input);
    assert_eq!(res, "<p><b>bold</b> text and <i>italic</i> word</p>");
}
