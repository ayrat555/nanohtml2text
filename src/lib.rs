// almost a line for line rewrite of https://github.com/k3a/html2text/blob/master/html2text.go
//
mod entity;

fn decode_named_entity(entity: &str) -> Option<char> {
    entity::ENTITIES
        .binary_search_by_key(&entity, |t| t.0)
        .map(|idx| entity::ENTITIES[idx].1)
        .ok()
}

fn parse_html_entity(ent_name: &str) -> Option<char> {
    let d = decode_named_entity(ent_name);
    if d.is_some() {
        return d;
    }

    let num = ent_name.strip_prefix("#")?;
    if num.chars().next()? == 'x' {
        u32::from_str_radix(&num[1..].to_lowercase(), 16)
    } else {
        // remaining string may be empty, but that will generate an Err(Empty)
        u32::from_str_radix(num, 10)
    }
    .ok()
    .filter(|n| !matches!(n, 9 | 10 | 13 | 32))
    .and_then(|n| char::from_u32(n))
}

fn html_entitities_to_text(s: &str) -> String {
    let mut out = String::new();

    // except for the first part, every part will have started with an ampersand
    // thus the start of the remaining parts is a HTML entity
    let mut parts = s.split('&');
    /*
    skip first part. if the string started with an ampersand, the first part
    will be an empty string

    if the string was empty, the first part will also be an empty string so its
    safe to unwrap
    */
    out.push_str(parts.next().unwrap());

    for part in parts {
        let end = part
            // entity can be terminated by semicolon or whitespace
            .find(|c: char| c.is_whitespace() || c == ';')
            // entity can also terminated by end of string or start of
            // another entity
            .unwrap_or_else(|| part.len());
        if let Some(entity) = parse_html_entity(&part[..end]) {
            out.push(entity);
            // get byte length of the char we did `find` above
            let skip = &part[end..]
                .chars()
                .next()
                // we know there is another character so its safe to unwrap
                .unwrap()
                .len_utf8();
            out.push_str(&part[end + skip..]);
        } else {
            out.push('&');
            out.push_str(part);
        }
    }

    out
}

/// Function to parse and handle the individual tags.
/// Assumes that there was a '<' before the given string
///
/// Returns the generated text and the byte length to skip.
fn handle_tag(s: &str) -> (String, usize) {
    let (tag, more) = match s.split_once('>') {
        Some((tag, more)) if !tag.is_empty() => (tag, more),
        _ => {
            // was not actually a tag, so reinsert the '<'
            return (String::from("<"), 0);
        }
    };

    let (name, attribs) = if let Some((name, attribs)) = tag.split_once(char::is_whitespace) {
        (name, Some(attribs))
    } else {
        (tag, None)
    };

    match name.to_lowercase().as_str() {
        "a" => {
            let href = attribs
                .and_then(|attribs| {
                    Some(
                        attribs
                            // check for the href and then discard everything before it
                            .split_once("href")?
                            .1
                            // there might be whitespace between 'href' and '='
                            .trim_start()
                            // check for and then discard the equal sign
                            .strip_prefix('=')?
                            // remove whitespace after the equal sign
                            .trim_start(),
                    )
                })
                .and_then(|href_value|
                    // find quoted string
                    match href_value.chars().next()? {
                        start @ '\'' | start @ '"' => {
                            let (end, _) = href_value
                                .char_indices()
                                .skip(1)
                                .find(|(_, c)| *c == start)?;
                            Some(href_value[1..end].to_string())
                        }
                        _ => None,
                    })
                .filter(|href| !href.starts_with("javascript:"))
                .map(|href| html_entitities_to_text(&href));
            // only use to_ascii_lowercase here so the byte offsets dont get
            // messed up from one uppercase symbol becoming two lowercase
            // symbols or something like that
            let more = more.to_ascii_lowercase();
            let end = more
                .find("</a")
                .map(|i| i + 3)
                .and_then(|end_tag| more[end_tag..].find('>').map(|i| end_tag + i + 1))
                .unwrap_or_else(|| more.len());
            (href.unwrap_or_default(), tag.len() + 1 + end)
        }
        "br" | "br/" | "li" | "/ol" | "/ul" => (String::from("\r\n"), tag.len() + 1),
        "p" | "h1" | "h2" | "h3" | "h4" | "h5" | "h6" | "/h1" | "/h2" | "/h3" | "/h4" | "/h5"
        | "/h6" => (String::from("\r\n\r\n"), tag.len() + 1),
        name @ "head" | name @ "script" | name @ "style" => {
            // silence tags

            // only use to_ascii_lowercase here so the byte offsets dont get
            // messed up from one uppercase symbol becoming two lowercase
            // symbols or something like that
            let more = more.to_ascii_lowercase();
            let end = more
                .find(&format!("</{}", name))
                .map(|i| i + 2 + name.len())
                .and_then(|end_tag| more[end_tag..].find('>').map(|i| i + end_tag + 1))
                .unwrap_or_else(|| more.len());
            (String::new(), tag.len() + 1 + end)
        }
        "!--" => {
            // HTML comment
            (String::new(), s.find("-->").map_or(s.len(), |n| n + 3))
        }
        // other/unknown tags are just discarded
        _ => (String::new(), tag.len() + 1),
    }
}
pub fn html2text(html: &str) -> String {
    // collapse spaces
    let html = html.split_whitespace().collect::<Vec<_>>().join(" ");

    let mut out = String::new();

    let mut i = 0;
    while i < html.len() {
        match html[i..].find('<') {
            None => {
                // no more tags in the input, done
                out += &html_entitities_to_text(&html[i..]);
                break;
            }
            Some(text_segment) => {
                if text_segment > 0 {
                    out += &html_entitities_to_text(&html[i..i + text_segment]);
                    i += text_segment;
                }
                i += 1; // skip the '<'
                let (s, advance) = handle_tag(&html[i..]);
                if !s.is_empty() {
                    if out.ends_with("\r\n\r\n") || out.is_empty() {
                        out += &s.trim_start();
                    } else {
                        out += &s;
                    }
                }
                i += advance;
            }
        }
    }

    out
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
