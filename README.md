My first foray into becoming a Rustacean  

Building a text editor in Rust following https://www.flenker.blog/hecto/


# TODO:
- be more consistent with saturating_add/sub vs +/-
- auto-indent (when starting a new line, indent to the same level as the previous line)
- maintain cursor position like most text editors when going to a shorter line then a longer line (maintain horizontal cursor position once returning to a line that can fit it)
- Upgrade termion to v2, see what new features it includes
- switch from unicode-segmentation package to grapheme package
