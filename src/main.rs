// almost a line for line rewrite of https://github.com/k3a/html2text/blob/master/html2text.go
fn main() {
    println!("Hello, world!");
}

fn write_space(s: &mut String) {}

fn html2text(input: &str) -> String {
    let in_len = input.len();
    let mut tag_start = 0;
    let mut in_ent = false;
    let mut bad_tag_stack_depth = 0;
    let mut should_output = true;
    let mut can_print_new_line = false;
    let mut out_buf = String::new();
    for (i, r) in input.chars().enumerate() {
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
            in_ent = false;
            // parse the entity name, max 10 chars
        }
    }
    out_buf
}
