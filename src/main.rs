// almost a line for line rewrite of https://github.com/k3a/html2text/blob/master/html2text.go
//
mod entity;
fn main() {
    println!("Hello, world!");
}

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

fn parse_html_entity(ent_name: &str) {
    // entities.binary_search_by(k
}

fn write_space(s: &mut String) {
    let b = s.as_bytes();
    if b.len() > 0 && b[b.len() - 1] != b' ' {
        s.push(' ');
    }
}

fn html2text(html: &str) -> String {
    let in_len = html.len();
    let mut tag_start = 0;
    let mut in_ent = false;
    let mut bad_tag_stack_depth = 0;
    let mut should_output = true;
    let mut can_print_new_line = false;
    let mut out_buf = String::new();
    for (i, r) in html.chars().enumerate() {
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
                // parseHTMLentity TODO
            }
        } else if r == '<' {
            // start of tag
            tag_start = i + 1;
            should_output = false;
            continue;
        } else if r == '>' { // end of tag
             // TODO
        }

        if should_output && bad_tag_stack_depth == 0 && !in_ent {
            can_print_new_line = true;
            out_buf.push(r);
        }
    }
    out_buf
}
