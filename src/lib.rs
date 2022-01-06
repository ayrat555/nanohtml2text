// almost a line for line rewrite of https://github.com/k3a/html2text/blob/master/html2text.go
//
mod entity;

const LBR: &str = "\r\n";
// stolen from https://github.com/veddan/rust-htmlescape/blob/master/src/decode.rs
fn decode_named_entity(entity: &str) -> Option<char> {
    match entity::ENTITIES.binary_search_by(|&(ent, _)| ent.cmp(entity)) {
        Err(..) => None,
        Ok(idx) => {
            let (_, c) = entity::ENTITIES[idx];
            Some(c)
        }
    }
}

const BAD_TAGS: [&str; 4] = ["head", "script", "style", "a"];

// awkward
fn parse_link(l: &str) -> Option<&str> {
    if l.starts_with("a") {
        let s: Vec<&str> = l.split("href=").collect();
        if s.len() > 1 {
            if s[1] != "" {
                if s[1].as_bytes()[0] == b'\'' {
                    let end = s[1][1..].bytes().position(|c| c == b'\'');
                    if let Some(p) = end {
                        return Some(&s[1][1..=p]);
                    }
                } else if s[1].as_bytes()[0] == b'"' {
                    let end = s[1][1..].bytes().position(|c| c == b'"');
                    if let Some(p) = end {
                        return Some(&s[1][1..=p]);
                    }
                }
            }
        }
    }
    None
}

fn is_bad_tag(t: &str) -> bool {
    let t = t.split_whitespace().next().unwrap();
    if BAD_TAGS.contains(&t) {
        return true;
    }
    false
}

// replacing regex
fn is_header(h: &str) -> bool {
    let mut b = h.as_bytes();
    if b.len() == 3 && b[0] == b'/' {
        b = &b[1..]
    }
    if b.len() == 2 && b[0] == b'h' {
        if b'1' <= b[1] && b[1] <= b'6' {
            return true;
        }
    }
    false
}

fn parse_html_entity(ent_name: &str) -> Option<char> {
    let d = decode_named_entity(ent_name);
    if d.is_some() {
        return d;
    }
    // rewriting without regex
    let lower = ent_name.to_lowercase();
    if lower.starts_with("#") && lower.len() > 1 {
        let parsed;
        if lower.as_bytes()[1] == b'x' && lower.len() > 2 {
            parsed = u32::from_str_radix(&lower[2..], 16).ok();
        } else {
            parsed = u32::from_str_radix(&lower[1..], 10).ok();
        }
        return parsed.and_then(|n| {
            if n == 9 || n == 10 || n == 13 || n > 32 {
                return char::from_u32(n);
            }
            return None;
        });
    }

    None
}

fn html_entitities_to_text(s: &str) -> String {
    let mut out = String::new();
    let mut in_ent = false;
    for (i, r) in s.char_indices() {
        if r == ';' && in_ent {
            in_ent = false;
            continue;
        } else if r == '&' {
            let mut ent_name = String::new();
            let mut is_ent = false;
            let mut chars = 0;
            for er in s[i + 1..].chars() {
                if er == ';' {
                    is_ent = true;
                    break;
                } else {
                    ent_name.push(er);
                }
                chars += 1;
                if chars == 10 {
                    break;
                }
            }
            if is_ent {
                if let Some(ent) = parse_html_entity(&ent_name) {
                    out.push(ent);
                    in_ent = true;
                    continue;
                }
            }
        }
        if !in_ent {
            out.push(r);
        }
    }
    out
}

fn write_space(s: &mut String) {
    let b = s.as_bytes();
    if b.len() > 0 && b[b.len() - 1] != b' ' {
        s.push(' ');
    }
}

pub fn html2text(html: &str) -> String {
    let in_len = html.len();
    let mut tag_start = 0;
    let mut in_ent = false;
    let mut bad_tag_stack_depth = 0;
    let mut should_output = true;
    let mut can_print_new_line = false;
    let mut out_buf = String::new();
    for (i, r) in html.char_indices() {
        if in_len > 0 && i == in_len - 1 {
            can_print_new_line = false
        }
        if r.is_whitespace() {
            if should_output && bad_tag_stack_depth == 0 && !in_ent {
                write_space(&mut out_buf);
            }
            continue;
        } else if r == ';' && in_ent {
            in_ent = false;
            continue;
        } else if r == '&' && should_output {
            let mut ent_name = String::new();
            let mut is_ent = false;
            let mut chars = 10;
            for er in html[i + 1..].chars() {
                if er == ';' {
                    is_ent = true;
                    break;
                } else {
                    ent_name.push(er);
                }
                chars += 1;
                if chars == 10 {
                    break;
                }
            }
            if is_ent {
                if let Some(ent) = parse_html_entity(&ent_name) {
                    out_buf.push(ent);
                    in_ent = true;
                }
            }
        } else if r == '<' {
            // start of tag
            tag_start = i + 1;
            should_output = false;
            continue;
        } else if r == '>' {
            should_output = true;
            let tag = &html[tag_start..i];
            let tag_name_lower = tag.to_lowercase();
            if tag_name_lower == "/ul" {
                out_buf.push_str(LBR);
            } else if tag_name_lower == "li" || tag_name_lower == "li/" {
                out_buf.push_str(LBR);
            } else if is_header(&tag_name_lower) {
                if can_print_new_line {
                    out_buf.push_str(LBR);
                    out_buf.push_str(LBR);
                }
                can_print_new_line = false;
            } else if tag_name_lower == "br" || tag_name_lower == "br/" {
                out_buf.push_str(LBR);
            } else if tag_name_lower == "p" || tag_name_lower == "/p" {
                if can_print_new_line {
                    out_buf.push_str(LBR);
                    out_buf.push_str(LBR);
                }
                can_print_new_line = false;
            } else if is_bad_tag(&tag_name_lower) {
                bad_tag_stack_depth += 1;
                // parse link
                if let Some(link) = parse_link(tag) {
                    if !link.contains("javascript:") {
                        out_buf.push_str(&html_entitities_to_text(link));
                        can_print_new_line = true;
                    }
                }
            } else if tag_name_lower.len() > 0
                && tag_name_lower.starts_with("/")
                && is_bad_tag(&tag_name_lower[1..])
            {
                bad_tag_stack_depth -= 1;
            }
            continue;
        }

        if should_output && bad_tag_stack_depth == 0 && !in_ent {
            can_print_new_line = true;
            out_buf.push(r);
        }
    }
    out_buf
}

#[cfg(test)]
mod tests {
    use super::*;
    const cases: &[(&str, &str)] = &[
        ("blah", "blah"),
        // links
        ("<div></div>", ""),
        ("<div>simple text</div>", "simple text"),
        ("click <a href=\"test\">here</a>", "click test"),
        ("click <a class=\"x\" href=\"test\">here</a>", "click test"),
        (
            "click <a href=\"ents/&apos;x&apos;\">here</a>",
            "click ents/'x'",
        ),
        ("click <a href=\"javascript:void(0)\">here</a>", "click "),
        (
            "click <a href=\"test\"><span>here</span> or here</a>",
            "click test",
        ),
        (
            "click <a href=\"http://bit.ly/2n4wXRs\">news</a>",
            "click http://bit.ly/2n4wXRs",
        ),
        ("<a rel=\"mw:WikiLink\" href=\"/wiki/yet#English\" title=\"yet\">yet</a>, <a rel=\"mw:WikiLink\" href=\"/wiki/not_yet#English\" title=\"not yet\">not yet</a>", "/wiki/yet#English, /wiki/not_yet#English"),

        // inlines
        ("strong <strong>text</strong>", "strong text"),
        ("some <div id=\"a\" class=\"b\">div</div>", "some div"),
        // lines breaks and spaces
        ("should    ignore more spaces", "should ignore more spaces"),
        ("should \nignore \r\nnew lines", "should ignore new lines"),
        ("a\nb\nc", "a b c"),
        ("two<br>line<br/>breaks", "two\r\nline\r\nbreaks"),
        ("<p>two</p><p>paragraphs</p>", "two\r\n\r\nparagraphs"),
        // Headers
        ("<h1>First</h1>main text", "First\r\n\r\nmain text"),
        (
            "First<h2>Second</h2>next section",
            "First\r\n\r\nSecond\r\n\r\nnext section",
        ),
        ("<h2>Second</h2>next section", "Second\r\n\r\nnext section"),
        (
            "Second<h3>Third</h3>next section",
            "Second\r\n\r\nThird\r\n\r\nnext section",
        ),
        ("<h3>Third</h3>next section", "Third\r\n\r\nnext section"),
        (
            "Third<h4>Fourth</h4>next section",
            "Third\r\n\r\nFourth\r\n\r\nnext section",
        ),
        ("<h4>Fourth</h4>next section", "Fourth\r\n\r\nnext section"),
        (
            "Fourth<h5>Fifth</h5>next section",
            "Fourth\r\n\r\nFifth\r\n\r\nnext section",
        ),
        ("<h5>Fifth</h5>next section", "Fifth\r\n\r\nnext section"),
        (
            "Fifth<h6>Sixth</h6>next section",
            "Fifth\r\n\r\nSixth\r\n\r\nnext section",
        ),
        ("<h6>Sixth</h6>next section", "Sixth\r\n\r\nnext section"),
        ("<h7>Not Header</h7>next section", "Not Headernext section"),
        // html entitites
        ("two&nbsp;&nbsp;spaces", "two  spaces"),
        ("&copy; 2017 K3A", "© 2017 K3A"),
        ("&lt;printtag&gt;", "<printtag>"),
        (
            "would you pay in &cent;, &pound;, &yen; or &euro;?",
            "would you pay in ¢, £, ¥ or €?",
        ),
        (
            "Tom & Jerry is not an entity",
            "Tom & Jerry is not an entity",
        ),
        ("this &neither; as you see", "this &neither; as you see"),
        (
            "list of items<ul><li>One</li><li>Two</li><li>Three</li></ul>",
            "list of items\r\nOne\r\nTwo\r\nThree\r\n",
        ),
        ("fish &amp; chips", "fish & chips"),
        (
            "&quot;I'm sorry, Dave. I'm afraid I can't do that.&quot; – HAL, 2001: A Space Odyssey",
            "\"I'm sorry, Dave. I'm afraid I can't do that.\" – HAL, 2001: A Space Odyssey",
        ),
        ("Google &reg;", "Google ®"),
        (
            "&#8268; decimal and hex entities supported &#x204D;",
            "⁌ decimal and hex entities supported ⁍",
        ),
        // Large entity
        ("&abcdefghij;", "&abcdefghij;"),
        // Numeric HTML entities
        (
            "&#39;single quotes&#39; and &#52765;",
            "'single quotes' and 츝",
        ),
        // full thml structure
        ("", ""),
        ("<html><head><title>Good</title></head><body>x</body>", "x"),
        (
            "we are not <script type=\"javascript\"></script>interested in scripts",
            "we are not interested in scripts",
        ),
        // custom html tags
        ("<aa>hello</aa>", "hello"),
        ("<aa >hello</aa>", "hello"),
        ("<aa x=\"1\">hello</aa>", "hello"),
    ];

    #[test]
    fn test_all() {
        for case in cases {
            assert_eq!(&html2text(case.0), case.1);
        }
    }
}
