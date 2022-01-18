nanohtml2text
=============

0-dependency library to convert HTML to text; an alternative to
https://crates.io/crates/html2text that doesn't use a full browser-grade HTML
parser

Based on https://github.com/k3a/html2text 

Primarily useful for displaying HTML emails as text. YMMV depending on the
structure of the HTML you're trying to convert. Built for the needs of
Crabmail: https://git.alexwennerberg.com/crabmail/

This library has one function, html2text, which takes a an html &str and
returns a plain text String

on crates.io:
https://crates.io/crates/nanohtml2text

Comes with a command line utility in main.rs to process from stdin if you want
to test/experiment with it

Contributing
------------
git-send-email or git-request-pull to my mailing list:
https://lists.flounder.online/patches/
