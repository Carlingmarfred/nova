# Nova Grammar (EBNF)

Normative. `~` = exception, `?` = optional, `*` = zero or more, `+` = one or more.

## 1. Program and declaration level

```ebnf
program        = { newline }, { top_item } ;
top_item       = attribute*, declaration ;
declaration    = fn_decl | class_decl | struct_decl | enum_decl
               | trait_decl | impl_decl | extend_decl
               | var_decl | const_decl | type_alias
               | mod_decl | import_decl | export_decl ;

attribute      = "@" , ident , [ attr_args ] ;
attr_args      = "(" , [ arg_list ] , ")" ;

fn_decl        = "fn" , ident , generics?, "(" , params? , ")" ,
                 [ "->" , type ] , block ;
generics       = "<" , generic_param , { "," , generic_param } , ">" ;
generic_param  = ident , [ ":" , bound_list ] , [ "=" , type ] ;
bound_list     = type , { "+" , type } ;          (* T: Drawable + Hashable *)

params         = param , { "," , param } , [ "," ] ;
param          = [ "mut" ] , ident , ":" , type , [ "=" , expression ]
               | "..." , ident , ":" , type       (* variadic *)
               | "**" , ident , ":" , type ;      (* keyword-collect *)

class_decl     = "class" , ident , generics? ,
                 [ ":" , type , { "," , trait_ref } ] , class_body ;
class_body     = "{" , { member } , "}" ;
member         = attribute*, ( field | method | init_decl | deinit_decl
                             | property | static_member ) ;
field          = [visibility], ident , ":" , type , [ "=" , expression ] ;
method         = [visibility] , [ "static" | "virtual" | "override" ] , fn_decl ;
init_decl      = "fn" , "init" , "(" , params? , ")" , block ;
deinit_decl    = "fn" , "deinit" , "(" , ")" , block ;
property       = ident , ":" , type , "{" ,
                     [ "get" , block ] , [ "set" , "(" , ident? , ")" , block ] ,
                 "}" ;

struct_decl    = "struct" , ident , generics? , struct_body ;
struct_body    = "{" , { field } , "}" ;

enum_decl      = "enum" , ident , generics? , "{" , variant , { "|" | "," , variant } , "}" ;
variant        = ident , [ "(" , type_list , ")" ] , [ "=" , expression ] ;

trait_decl     = "trait" , ident , generics? , [ ":" , trait_ref_list ] , trait_body ;
trait_body     = "{" , { trait_item } , "}" ;
trait_item     = fn_signature | default_method | assoc_type | assoc_const ;

impl_decl      = "impl" , generics? , trait_ref , "for" , type , impl_body ;
extend_decl    = "extend" , type , extend_body ;   (* extension methods *)

mod_decl       = "mod" , ident , [ "{" , { top_item } , "}" ] ;
import_decl    = "import" , path , [ "as" , ident ]
               | "from" , path , "import" , ident_list ;
export_decl    = "export" ;
var_decl       = [ "let" | "var" ] , pattern , [ ":" , type ] , "=" , expression ;
const_decl     = "const" , ident , [ ":" , type ] , "=" , expression ;
type_alias     = "type" , ident , generics? , "=" , type ;
```

## 2. Statements

```ebnf
block          = "{" , { statement } , "}" ;
statement      = attribute*, (
                   var_decl | expr_stmt | assignment
                 | if_stmt | while_stmt | loop_stmt | for_stmt
                 | match_stmt | return_stmt | yield_stmt
                 | break_stmt | continue_stmt | labeled_stmt
                 | use_stmt | defer_stmt | try_stmt | throw_stmt
                 | unsafe_block | parallel_block | async_block ) ;
labeled_stmt   = ident , ":" , ( for_stmt | while_stmt | loop_stmt ) ;
if_stmt        = "if" , expression , block , { "else" , "if" , expression , block } , [ "else" , block ] ;
while_stmt     = "while" , expression , block ;
loop_stmt      = "loop" , block ;
for_stmt       = "for" , pattern , "in" , expression , block ;
match_stmt     = "match" , expression , "{", match_arm, { ",", match_arm }, "}" ;
match_arm      = pattern , [ "where" , expression ] , "=>" , expression | block ;
return_stmt    = "return" , [ expression ] ;
use_stmt       = "use" , pattern , "=" , expression , block ;
defer_stmt     = "defer" , expression ;
try_stmt       = "try" , block , { catch_clause } , [ "finally" , block ] ;
catch_clause   = "catch" , [ ident , [ ":" , type ] ] , block ;
throw_stmt     = "throw" , expression ;
unsafe_block   = "unsafe" , block ;
parallel_block = "parallel" , [ "(" , par_opts , ")" ] , block ;
async_block    = "async" , block ;
```

Newline rule (lexer): a statement ends at a newline unless the line ends with an operator, comma, open bracket/brace or `=>`, or the next line starts with a closing bracket / binary operator. `;` can always be used as a separator.

## 3. Patterns

```ebnf
pattern        = or_pattern , [ "where" , expression ] ;   (* only in match arms *)
or_pattern     = primary_pattern , { "|" , primary_pattern } ;
primary_pattern= literal
               | range
               | "_" 
               | binding , [ "@" , subpattern ]
               | tuple_pattern
               | array_pattern
               | struct_pattern
               | variant_pattern
               | "is" , type ;
binding        = [ "let" ] , ident ;
tuple_pattern  = "(" , pattern , "," , pattern , { "," , pattern } , ")" ;
array_pattern  = "[" , [ elem_pat , { "," , elem_pat } , [ "," , ".." , ident? ] ] , "]" ;
struct_pattern = type_path , "{" , [ ident , [ ":" , pattern ] , { "," , ... } ] , "}" ;
variant_pattern= type_path? , "::"? , ident , [ "(" , pattern_list , ")" ] ;
subpattern     = literal | range | struct_pattern | variant_pattern | array_pattern ;
range          = int_literal , (".." | "..=") , int_literal ;
```

## 4. Expressions and precedence

```ebnf
expression     = assignment_expr ;
assignment_expr= or_expr , [ assign_op , assignment_expr ] ;    (* right-assoc. *)
assign_op      = "=" | "+=" | "-=" | "*=" | "/=" | "%=" | "//=" | "**="
               | "&=" | "|=" | "^=" | "<<=" | ">>=" | "?=" ;
or_expr        = and_expr , { ("||"|"or") , and_expr } ;
and_expr       = not_expr , { ("&&"|"and") , not_expr } ;
not_expr       = [ "!"|"not" ] , comparison ;
comparison     = bitwise_or , [ compare_op , bitwise_or ] ;     (* non-chainable! *)
compare_op     = "==" | "!=" | "<" | "<=" | ">" | ">=" | "<=>" 
               | "is" [ "not" ] , type_or_trait | [ "not" ] , "in" ;
bitwise_or     = bitwise_xor , { ("|"|"^") , bitwise_xor } ;
bitwise_and    = shift , { "&" , shift } ;
shift          = additive , { ("<<"|">>") , additive } ;
additive       = multiplicative , { ("+"|"-") , multiplicative } ;
multiplicative = power , { ("*"|"/"|"%"|"//") , power } ;
power          = unary , [ "**" , power ] ;                     (* right-assoc. *)
unary          = [ "-" | "+" | "~" ] , unary
               | "*" unary              (* deref, unsafe *)
               | "&" unary              (* addr-of, unsafe *)
               | postfix ;
postfix        = primary , { postfix_op } ;
postfix_op     = "(" , [ call_args ] , ")"
               | "[" , index_or_slice , "]"
               | "." , ident , [ "(" , [call_args] , ")" ]
               | "?." , ident , [ "(" , [call_args] , ")" ]
               | "?"                    (* error propagation *)
               | "!"                    (* unwrap-or-panic *)
               | "as" , type ;
primary        = literal | ident | type_path | "(" , expression , ")"
               | tuple | array_lit | map_lit | set_lit
               | lambda | if_expr | match_expr
               | block                  (* block expression *)
               | comprehension ;
lambda         = ident , "=>" , expression
               | "(" , lambda_params? , ")" , "=>" , ( expression | block )
               | "[" , capture_list , "]" , lambda_params , "=>" , body ;
comprehension  = "[" , expr_or_kv , "for" , pattern , "in" , expression ,
                 { "for" , ... } , { "if" , expression } , "]"
               | "{" , expr , "for" ... , "}" ;                (* set *)
type_path      = ident , { "::" , ident } , [ generics_args ] ;
```

Slice: `[ start? , ":" , end? , [ ":" , step? ] ]` or a Range object `a..b`; negative indexing via `^n`.

## 5. Typer

```ebnf
type           = union_type ;
union_type     = nullable_type , { "|" , nullable_type } ;
nullable_type  = postfix_type , [ "?" ] ;
postfix_type   = base_type , { "[" , [ type ] , "]" } ;   (* Array<T>, [T;N] via as *)
base_type      = prim_type | type_path | function_type | tuple_type
               | "(" , type , ")" | "&" type | "*" type ; (* ref/raw, unsafe *)
function_type  = "fn" , "(" , [ type_list ] , ")" , [ "->" , type ] ;
tuple_type     = "(" , type , "," , type , { "," , type } , ")" ;
prim_type      = "i8|i16|i32|i64|i128|u8|u16|u32|u64|u128"
               | "isize|usize|f32|f64|bool|char|String|BigInt|dynamic|()" ;
```

## 6. Lexical (references)

See lexical.md for tokens/literals.
