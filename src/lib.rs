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

/// Convert some HTML to plain text. Only some simple HTML tags are handled:
/// - `a` tags are transformed to their href attribute value
/// - paragraph, linebreak, heading, list, and list item tags insert different
///   amounts of line breaks.
/// - HTML comments as well as `head`, `script` and `style` are completely
///   discarded, including their content
/// - unknown tags are skipped, but their content is printed
///
/// HTML named entities will be replaced with the respecive Unicode code point,
/// and whitespace will be collapsed as is usual in HTML.
///
/// The resulting string will have CRLF line endings.
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

    macro_rules! test {
        ($name:ident, $from:literal, $to:literal $(,)?) => {
            #[test]
            fn $name() {
                assert_eq!(&html2text($from), $to);
            }
        };
        ($($name:ident: $from:literal to $to:literal,)* $(,)?) => {
            $(test!{$name, $from, $to})*
        };
    }

    test! {
        plaintext: "blah" to "blah",
        tag: "<div></div>" to "",
        tag_contents: "<div>simple text</div>" to "simple text",
        // links
        link:
            "click <a href=\"test\">here</a>"
            to "click test",
        links_ignore_attributes:
            "click <a class=\"x\" href=\"test\">here</a>"
            to "click test",
        link_entities_in_url:
            "click <a href=\"ents/&apos;x&apos;\">here</a>"
            to "click ents/'x'",
        link_javascript:
            "click <a href=\"javascript:void(0)\">here</a>"
            to "click ",
        link_ignore_content_tags:
            "click <a href=\"test\"><span>here</span> or here</a>"
            to "click test",
        link_absolute_url:
            "click <a href=\"http://bit.ly/2n4wXRs\">news</a>"
            to "click http://bit.ly/2n4wXRs",
        link_ignore_attributes_2:
            "<a rel=\"mw:WikiLink\" href=\"/wiki/yet#English\" title=\"yet\">yet</a>, <a rel=\"mw:WikiLink\" href=\"/wiki/not_yet#English\" title=\"not yet\">not yet</a>"
            to "/wiki/yet#English, /wiki/not_yet#English",
        // inlines
        ignore_inline:
            "strong <strong>text</strong>"
            to "strong text",
        ignore_inline_attributes:
            "some <div id=\"a\" class=\"b\">div</div>"
            to "some div",
        // lines breaks and spaces
        collapse_spaces:
            "should    ignore more spaces" to "should ignore more spaces",
        collapse_linebreaks:
            "a\nb\nc" to "a b c",
        collapse_mixed:
            "should \nignore \r\nnew lines" to "should ignore new lines",
        br_tag:
            "two<br>line<br/>breaks" to "two\r\nline\r\nbreaks",
        paragraph:
            "<p>two</p><p>paragraphs</p>" to "two\r\n\r\nparagraphs",
        // Headers
        h1:
            "<h1>First</h1>main text" to "First\r\n\r\nmain text",
        h2_inline:
            "First<h2>Second</h2>next section"
            to "First\r\n\r\nSecond\r\n\r\nnext section",
        h2:
            "<h2>Second</h2>next section" to "Second\r\n\r\nnext section",
        h3_inline:
            "Second<h3>Third</h3>next section"
            to "Second\r\n\r\nThird\r\n\r\nnext section",
        h3:
            "<h3>Third</h3>next section" to "Third\r\n\r\nnext section",
        h4_inline:
            "Third<h4>Fourth</h4>next section"
            to "Third\r\n\r\nFourth\r\n\r\nnext section",
        h4:
            "<h4>Fourth</h4>next section" to "Fourth\r\n\r\nnext section",
        h5_inline:
            "Fourth<h5>Fifth</h5>next section"
            to "Fourth\r\n\r\nFifth\r\n\r\nnext section",
        h5:
            "<h5>Fifth</h5>next section" to "Fifth\r\n\r\nnext section",
        h6_inline:
            "Fifth<h6>Sixth</h6>next section"
            to "Fifth\r\n\r\nSixth\r\n\r\nnext section",
        h6:
            "<h6>Sixth</h6>next section" to "Sixth\r\n\r\nnext section",
        no_h7:
            "<h7>Not Header</h7>next section" to "Not Headernext section",
        // html entitites
        entity_nbsp:
            "two&nbsp;&nbsp;spaces" to "two  spaces",
        entity_copy:
            "&copy; 2017 K3A" to "© 2017 K3A",
        entity_tag:
            "&lt;printtag&gt;" to "<printtag>",
        entity_currencies:
            "would you pay in &cent;, &pound;, &yen; or &euro;?"
            to "would you pay in ¢, £, ¥ or €?",
        ampersand_not_entity:
            "Tom & Jerry is not an entity" to "Tom & Jerry is not an entity",
        entity_unknown:
            "this &neither; as you see" to "this &neither; as you see",
        entity_amp:
            "fish &amp; chips" to "fish & chips",
        unordered_list:
            "list of items<ul><li>One</li><li>Two</li><li>Three</li></ul>"
            to "list of items\r\nOne\r\nTwo\r\nThree\r\n",
        entity_quot:
            "&quot;I'm sorry, Dave. I'm afraid I can't do that.&quot; – HAL, 2001: A Space Odyssey"
            to "\"I'm sorry, Dave. I'm afraid I can't do that.\" – HAL, 2001: A Space Odyssey",
        entity_reg:
            "Google &reg;" to "Google ®",
        // Large entity
        entity_large_unknown:
            "&abcdefghij;" to "&abcdefghij;",
        // Numeric HTML entities
        entity_numeric:
            "&#8268; decimal and hex entities supported &#x204D;"
            to "⁌ decimal and hex entities supported ⁍",
        entity_numeric_2:
            "&#39;single quotes&#39; and &#52765;"
            to "'single quotes' and 츝",
        // full thml structure
        empty: "" to "",
        full_html:
            "<html><head><title>Good</title></head><body>x</body>" to "x",
        ignore_script:
            "we are not <script type=\"javascript\"></script>interested in scripts"
            to "we are not interested in scripts",
        // custom html tags
        ignore_unknown_tag:
            "<aa>hello</aa>" to "hello",
        ignore_unknown_tag_whitespace:
            "<aa >hello</aa>" to "hello",
        ignore_unknown_tag_attributes:
            "<aa x=\"1\">hello</aa>" to "hello",
    }
}
