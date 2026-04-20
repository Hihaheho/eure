; Comments
(comment) @comment
(block_comment) @comment.block

; Sections: @ keyword and name
(section "@" @keyword.operator)
(section (key_path (key_segment (key_identifier (identifier) @label))))

; Extension namespace: $name
(extension_key "$" @operator)
(extension_key (identifier) @tag)

; Binding property keys
(binding (key_path (key_segment (key_identifier (identifier) @property))))

; Map bind operator => and assignment =
"=>" @operator
"=" @operator
":" @punctuation.delimiter

; Strings
(escaped_string) @string
(literal_string) @string
(delim_string_1) @string
(delim_string_2) @string
(delim_string_3) @string

; Numbers
(integer) @number
(float) @number
(inf) @number
(nan) @number

; Booleans and null
(boolean) @boolean
(null) @constant.builtin

; Holes
(hole) @variable.special

; Inline code
(inline_code_simple) @string.special
(delim_code_content) @string.special

; Code block delimiters, language, and content
(code_block_fence) @punctuation.special
(code_block_lang) @string.special
(code_block_content) @string.special

; Brackets and delimiters
["[" "]" "(" ")" "{" "}"] @punctuation.bracket
["," "."] @punctuation.delimiter

; String continuation backslash
"\\" @operator

; Special markers
"@" @keyword.operator
"$" @operator
"#" @punctuation.special
