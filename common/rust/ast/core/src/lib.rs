#![feature(type_alias_impl_trait)]
#![feature(generators, generator_trait)]

use prelude::*;

use ast_macros::*;
use serde::{Serialize, Deserialize};
use serde::ser::{Serializer, SerializeStruct};
use serde::de::{Deserializer, Visitor};
use shapely::*;
use uuid::Uuid;

pub type Stream<T> = Vec<T>;

// ==============
// === Errors ===
// ==============

/// Exception raised by macro-generated TryFrom methods that try to "downcast"
/// enum type to its variant subtype if different constructor was used.
#[derive(Display, Debug, Fail)]
pub struct WrongEnum { pub expected_con: String }

// ============
// === Tree ===
// ============

/// A tree structure where each node may store value of `K` and has arbitrary
/// number of children nodes, each marked with a single `K`.
///
/// It is used to describe ambiguous macro match.
#[derive(Eq, PartialEq, Debug, Serialize, Deserialize)]
pub struct Tree<K,V> {
    pub value    : Option<V>,
    pub branches : Vec<(K, Tree<K,V>)>,
}

// ===============
// === Shifted ===
// ===============

/// A value of type `T` annotated with offset value `off`.
#[derive(Eq, PartialEq, Debug, Serialize, Deserialize, Shrinkwrap)]
#[shrinkwrap(mutable)]
pub struct Shifted<T> {
    #[shrinkwrap(main_field)]
    pub wrapped : T,
    pub off     : usize,
}

/// A non-empty sequence of `T`s interspersed by offsets.
#[derive(Eq, PartialEq, Debug, Serialize, Deserialize)]
pub struct ShiftedVec1<T> {
    pub head: T,
    pub tail: Vec<Shifted<T>>
}


// =============
// === Layer ===
// =============

// === Trait ===

/// Types that can wrap a value of given `T`.
///
/// Same API as `From`, however not reflexive.
pub trait Layer<T> {
    fn layered(t: T) -> Self;
}

impl<T> From<T> for Layered<T> {
    fn from(t: T) -> Self {  Layered::layered(t) }
}

// === Layered ===

/// A trivial `Layer` type that is just a strongly typed wrapper over `T`.
#[derive(Debug)]
#[derive(Shrinkwrap)]
#[shrinkwrap(mutable)]
pub struct Layered<T>(pub T);

impl<T> Layer<T> for Layered<T> {
    fn layered(t: T) -> Self { Layered(t) }
}

// ============
// === Unit ===
// ============

/// A unit type defined as an empty struct.
///
/// Because it is defined using {} syntax, serde_json will serialize it to
/// an empty object rather than null node. This is to workaround issue with
/// using units in `Option`, reported here:
/// https://github.com/serde-rs/serde/issues/1690
#[ast_node] pub struct Unit{}


// ===========
// === AST ===
// ===========

/// The primary class for Enso Abstract Syntax Tree.
///
/// This implementation is paired with AST implementation for Scala. Any changes
/// to either of the implementation need to be applied to the other one as well.
///
/// Each AST node is annotated with span and an optional ID.
#[derive(Eq, PartialEq, Debug, Shrinkwrap)]
#[shrinkwrap(mutable)]
pub struct Ast {
    pub wrapped: Rc<WithID<WithSpan<Shape<Ast>>>>
}

impl Clone for Ast {
    fn clone(&self) -> Self {
        Ast { wrapped: self.wrapped.clone() }
    }
}

/// Iterates over all child nodes (including self).
pub fn iterate_subtree<T>(ast:T) -> impl Iterator<Item=T::Item>
where T: IntoIterator<Item=T> + Copy {
    let mut generator = move || {
        let mut nodes:Vec<T> = vec![ast];
        while !nodes.is_empty() {
            let ast = nodes.pop().unwrap();
            for child in ast.into_iter() {
                nodes.push(child)
            }
            yield ast;
        }
    };

    shapely::GeneratingIterator(generator)
}

impl<'t> IntoIterator for &'t Ast {
    type Item = &'t Ast;
    type IntoIter = impl Iterator<Item=&'t Ast>;

    fn into_iter(self) -> Self::IntoIter {
        self.shape().into_iter()
    }
}

impl Ast {
    pub fn shape(&self) -> &Shape<Ast> {
        self
    }

    /// Wraps given shape with an optional ID into Ast. Span will ba
    /// automatically calculated based on Shape.
    pub fn new<S: Into<Shape<Ast>>>(shape: S, id: Option<ID>) -> Ast {
        let shape: Shape<Ast> = shape.into();
        let span = shape.span();
        Ast::new_with_span(shape, id, span)
    }

    pub fn new_with_span<S: Into<Shape<Ast>>>
    (shape: S, id: Option<ID>, span: usize) -> Ast {
        let shape     = shape.into();
        let with_span = WithSpan { wrapped: shape,     span };
        let with_id   = WithID   { wrapped: with_span, id   };
        Ast { wrapped: Rc::new(with_id) }
    }

    /// Iterates over all child nodes (including self).
    pub fn traverse(&self) -> impl Iterator<Item=&Ast> {
        fn is_ii<T: IntoIterator>() {}
        is_ii::<&Ast>();

        iterate_subtree(self)
//        let mut generator = move || {
//            let mut nodes:Vec<&Ast> = vec![self];
//            while !nodes.is_empty() {
//                let ast = nodes.pop().unwrap();
//                for child in ast.into_iter() {
//                    nodes.push(child)
//                }
//                yield ast;
//            }
//        };
//
//        shapely::GeneratingIterator(generator)
    }
}

impl HasSpan for Ast {
    fn span(&self) -> usize {
        self.wrapped.span()
    }
}

/// Fills `id` with `None` by default.
impl<T: Into<Shape<Ast>>>
From<T> for Ast {
    fn from(t: T) -> Self {
        let id = None;
        Ast::new(t, id)
    }
}

// Serialization & Deserialization //

/// Literals used in `Ast` serialization and deserialization.
pub mod ast_schema {
    pub const STRUCT_NAME: &str      = "Ast";
    pub const SHAPE:       &str      = "shape";
    pub const ID:          &str      = "id";
    pub const SPAN:        &str      = "span";
    pub const FIELDS:      [&str; 3] = [SHAPE, ID, SPAN];
    pub const COUNT:       usize     = FIELDS.len();
}

impl Serialize for Ast {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where S: Serializer {
        use ast_schema::*;
        let mut state = serializer.serialize_struct(STRUCT_NAME, COUNT)?;
        state.serialize_field(SHAPE, &self.shape())?;
        if self.id.is_some() {
            state.serialize_field(ID, &self.id)?;
        }
        state.serialize_field(SPAN,  &self.span)?;
        state.end()
    }
}

/// Type to provide serde::de::Visitor to deserialize data into `Ast`.
struct AstDeserializationVisitor;

impl<'de> Visitor<'de> for AstDeserializationVisitor {
    type Value = Ast;

    fn expecting
    (&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        use ast_schema::*;
        write!(formatter, "an object with `{}` and `{}` fields", SHAPE, SPAN)
    }

    fn visit_map<A>
    (self, mut map: A) -> Result<Self::Value, A::Error>
    where A: serde::de::MapAccess<'de>, {
        use ast_schema::*;

        let mut shape: Option<Shape<Ast>> = None;
        let mut id:    Option<Option<ID>> = None;
        let mut span:  Option<usize>      = None;

        while let Some(key) = map.next_key()? {
            match key {
                SHAPE => shape = Some(map.next_value()?),
                ID    => id    = Some(map.next_value()?),
                SPAN  => span  = Some(map.next_value()?),
                _     => {},
            }
        }

        let shape = shape.ok_or(serde::de::Error::missing_field(SHAPE))?;
        let id    = id.unwrap_or(None); // allow missing `id` field
        let span  = span.ok_or(serde::de::Error::missing_field(SPAN))?;
        Ok(Ast::new_with_span(shape, id, span))
    }
}

impl<'de> Deserialize<'de> for Ast {
    fn deserialize<D>(deserializer: D) -> Result<Ast, D::Error>
    where D: Deserializer<'de> {
        use ast_schema::FIELDS;
        let visitor = AstDeserializationVisitor;
        deserializer.deserialize_struct("AstOf", &FIELDS, visitor)
    }
}




// =============
// === Shape ===
// =============

/// Defines shape of the subtree. Parametrized by the child node type `T`.
///
/// Shape describes names of children and spacing between them.
#[ast(flat)] pub enum Shape<T> {
    Unrecognized  { str : String   },
    InvalidQuote  { quote: Builder },
    InlineBlock   { quote: Builder },

    // === Identifiers ===
    Blank         { },
    Var           { name : String            },
    Cons          { name : String            },
    Opr           { name : String            },
    Mod           { name : String            },
    InvalidSuffix { elem : T, suffix: String },

    // === Number ===
    Number        { base: Option<String>, int: String },
    DanglingBase  { base: String                      },

    // === Text ===
    TextLineRaw   { text   : Vec<SegmentRaw>                  },
    TextLineFmt   { text   : Vec<SegmentFmt<T>>               },
    TextBlockRaw  { text   : Vec<TextBlockLine<SegmentRaw>>
                  , spaces : usize
                  , offset : usize                            },
    TextBlockFmt  { text   : Vec<TextBlockLine<SegmentFmt<T>>>
                  , spaces : usize
                  , offset : usize                            },
    TextUnclosed  { line   : TextLine<T>                      },

    // === Applications ===
    Prefix        { func : T,  off : usize, arg : T                         },
    Infix         { larg : T, loff : usize, opr : T, roff : usize, rarg : T },
    SectionLeft   {  arg : T,  off : usize, opr : T                         },
    SectionRight  {                         opr : T,  off : usize,  arg : T },
    SectionSides  {                         opr : T                         },

    // === Module ===
    Module        { lines       : Vec<BlockLine<Option<T>>>  },
    Block         { ty          : BlockType
                  , indent      : usize
                  , empty_lines : Vec<usize>
                  , first_line  : BlockLine<T>
                  , lines       : Vec<BlockLine<Option<T>>>
                  , is_orphan   : bool                       },

    // === Macros ===
    Match         { pfx      : Option<MacroPatternMatch<Shifted<Ast>>>
                  , segs     : ShiftedVec1<MacroMatchSegment<T>>
                  , resolved : Ast                                     },
    Ambiguous     { segs     : ShiftedVec1<MacroAmbiguousSegment>
                  , paths    : Tree<Ast, Unit>                         },

    // === Spaceless AST ===
    Comment       (Comment),
    Import        (Import<T>),
    Mixfix        (Mixfix<T>),
    Group         (Group<T>),
    Def           (Def<T>),
    Foreign       (Foreign),
}


// ===============
// === Builder ===
// ===============

#[ast(flat)]
pub enum Builder {
    Empty,
    Letter{char: char},
    Space {span: usize},
    Text  {str : String},
    Seq   {first: Rc<Builder>, second: Rc<Builder>},
}


// ============
// === Text ===
// ============

// === Text Block Lines ===
#[ast] pub struct TextBlockLine<T> {
    pub empty_lines: Vec<usize>,
    pub text       : Vec<T>
}

#[ast(flat)]
#[derive(HasSpan)]
pub enum TextLine<T> {
    TextLineRaw(TextLineRaw),
    TextLineFmt(TextLineFmt<T>),
}

// === Text Segments ===
#[ast(flat)] pub enum SegmentRaw {
    SegmentPlain    (SegmentPlain),
    SegmentRawEscape(SegmentRawEscape),
}

#[ast(flat)] pub enum SegmentFmt<T> {
    SegmentPlain    (SegmentPlain    ),
    SegmentRawEscape(SegmentRawEscape),
    SegmentExpr     (SegmentExpr<T>  ),
    SegmentEscape   (SegmentEscape   ),
}

#[ast_node] pub struct SegmentPlain     { pub value: String    }
#[ast_node] pub struct SegmentRawEscape { pub code : RawEscape }
#[ast_node] pub struct SegmentExpr<T>   { pub value: Option<T> }
#[ast_node] pub struct SegmentEscape    { pub code : Escape    }

// === Text Segment Escapes ===
#[ast(flat)] pub enum RawEscape {
    Unfinished { },
    Invalid    { str: char },
    Slash      { },
    Quote      { },
    RawQuote   { },
}

#[ast_node] pub enum Escape {
    Character{c     :char            },
    Control  {name  :String, code: u8},
    Number   {digits:String          },
    Unicode16{digits:String          },
    Unicode21{digits:String          },
    Unicode32{digits:String          },
}


// =============
// === Block ===
// =============

#[ast_node] pub enum   BlockType     { Continuous { } , Discontinuous { } }
#[ast]      pub struct BlockLine <T> { pub elem: T, pub off: usize }


// =============
// === Macro ===
// =============

#[ast] pub struct MacroMatchSegment<T> {
    pub head : Ast,
    pub body : MacroPatternMatch<Shifted<T>>
}

#[ast] pub struct MacroAmbiguousSegment {
    pub head: Ast,
    pub body: Option<Shifted<Ast>>
}

pub type MacroPattern = Rc<MacroPatternRaw>;
#[ast] pub enum MacroPatternRaw {

    // === Boundary Patterns ===
    Begin   { },
    End     { },

    // === Structural Patterns ===
    Nothing { },
    Seq     { pat1 : MacroPattern , pat2    : MacroPattern                    },
    Or      { pat1 : MacroPattern , pat2    : MacroPattern                    },
    Many    { pat  : MacroPattern                                             },
    Except  { not  : MacroPattern, pat      : MacroPattern                    },

    // === Meta Patterns ===
    Build   { pat  : MacroPattern                                             },
    Err     { msg  : String       , pat     : MacroPattern                    },
    Tag     { tag  : String       , pat     : MacroPattern                    },
    Cls     { cls  : PatternClass , pat     : MacroPattern                    },

    // === Token Patterns ===
    Tok     { spaced : Spaced     , ast     : Ast                             },
    Blank   { spaced : Spaced                                                 },
    Var     { spaced : Spaced                                                 },
    Cons    { spaced : Spaced                                                 },
    Opr     { spaced : Spaced     , max_prec : Option<usize>                  },
    Mod     { spaced : Spaced                                                 },
    Num     { spaced : Spaced                                                 },
    Text    { spaced : Spaced                                                 },
    Block   { spaced : Spaced                                                 },
    Macro   { spaced : Spaced                                                 },
    Invalid { spaced : Spaced                                                 },
}

#[ast] pub enum PatternClass { Normal, Pattern }
pub type Spaced = Option<bool>;

#[derive(Eq, PartialEq, Hash, Debug, Serialize, Deserialize)]
pub enum Either<L,R> { Left{value: L}, Right{value: R} }
pub type Switch<T> = Either<T,T>;

pub type MacroPatternMatch<T> = Rc<MacroPatternMatchRaw<T>>;
#[ast] pub enum MacroPatternMatchRaw<T> {

    // === Boundary Matches ===
    Begin   { pat: MacroPatternRawBegin },
    End     { pat: MacroPatternRawEnd   },

    // === Structural Matches ===
    Nothing { pat: MacroPatternRawNothing                                     },
    Seq     { pat: MacroPatternRawSeq     , elem: (MacroPatternMatch<T>,
                                                   MacroPatternMatch<T>)      },
    Or      { pat: MacroPatternRawOr      , elem: Switch<MacroPatternMatch<T>>},
    Many    { pat: MacroPatternRawMany    , elem: Vec<MacroPatternMatch<T>>   },
    Except  { pat: MacroPatternRawExcept  , elem: MacroPatternMatch<T>        },

    // === Meta Matches ===
    Build   { pat: MacroPatternRawBuild   , elem: T                           },
    Err     { pat: MacroPatternRawErr     , elem: T                           },
    Tag     { pat: MacroPatternRawTag     , elem: MacroPatternMatch<T>        },
    Cls     { pat: MacroPatternRawCls     , elem: MacroPatternMatch<T>        },

    // === Token Matches ===
    Tok     { pat: MacroPatternRawTok     , elem: T                           },
    Blank   { pat: MacroPatternRawBlank   , elem: T                           },
    Var     { pat: MacroPatternRawVar     , elem: T                           },
    Cons    { pat: MacroPatternRawCons    , elem: T                           },
    Opr     { pat: MacroPatternRawOpr     , elem: T                           },
    Mod     { pat: MacroPatternRawMod     , elem: T                           },
    Num     { pat: MacroPatternRawNum     , elem: T                           },
    Text    { pat: MacroPatternRawText    , elem: T                           },
    Block   { pat: MacroPatternRawBlock   , elem: T                           },
    Macro   { pat: MacroPatternRawMacro   , elem: T                           },
    Invalid { pat: MacroPatternRawInvalid , elem: T                           },

}


// =============================================================================
// === Spaceless AST ===========================================================
// =============================================================================

#[ast] pub struct Comment {
    pub lines: Vec<String>
}

#[ast] pub struct Import<T> {
    pub path: Vec<T> // Cons inside
}

#[ast] pub struct Mixfix<T> {
    pub name: Vec<T>,
    pub args: Vec<T>,
}

#[ast] pub struct Group<T> {
    pub body: Option<T>,
}

#[ast] pub struct Def<T> {
    pub name: T, // being with Cons
    pub args: Vec<T>,
    pub body: Option<T>
}

#[ast] pub struct Foreign {
    pub indent : usize,
    pub lang   : String,
    pub code   : Vec<String>
}


// ===========
// === AST ===
// ===========

// === HasSpan ===

/// Things that can be asked about their span.
pub trait HasSpan {
    fn span(&self) -> usize;
}

/// Counts codepoints.
impl HasSpan for char {
    fn span(&self) -> usize {
        1
    }
}

/// Counts codepoints.
impl HasSpan for String {
    fn span(&self) -> usize {
        self.as_str().span()
    }
}

/// Counts codepoints.
impl HasSpan for &str {
    fn span(&self) -> usize {
        self.chars().count()
    }
}

impl<T: HasSpan> HasSpan for Option<T> {
    fn span(&self) -> usize {
        self.as_ref().map_or(0, |wrapped| wrapped.span())
    }
}

impl<T: HasSpan> HasSpan for Vec<T> {
    fn span(&self) -> usize {
        let spans = self.iter().map(|elem| elem.span());
        spans.sum()
    }
}

impl<T: HasSpan> HasSpan for Rc<T> {
    fn span(&self) -> usize {
        self.deref().span()
    }
}

// === WithID ===

pub type ID = Uuid;

pub trait HasID {
    fn id(&self) -> Option<ID>;
}

#[derive(Eq, PartialEq, Debug, Shrinkwrap, Serialize, Deserialize)]
#[shrinkwrap(mutable)]
pub struct WithID<T> {
    #[shrinkwrap(main_field)]
    #[serde(flatten)]
    pub wrapped: T,
    pub id: Option<ID>
}

impl<T> HasID for WithID<T>
    where T: HasID {
    fn id(&self) -> Option<ID> {
        self.id
    }
}

impl<T, S:Layer<T>>
Layer<T> for WithID<S> {
    fn layered(t: T) -> Self {
        WithID { wrapped: Layer::layered(t), id: None }
    }
}

impl<T> HasSpan for WithID<T>
where T: HasSpan {
    fn span(&self) -> usize {
        self.deref().span()
    }
}

// === WithSpan ===

/// Stores a value of type `T` and information about its span.
///
/// Even if `T` is `Spanned`, keeping `span` variable is desired for performance
/// purposes.
#[derive(Eq, PartialEq, Debug, Shrinkwrap, Serialize, Deserialize)]
#[shrinkwrap(mutable)]
pub struct WithSpan<T> {
    #[shrinkwrap(main_field)]
    #[serde(flatten)]
    pub wrapped: T,
    pub span: usize
}

//impl<T> HasSpan for WithSpan<T> {
//    fn span(&self) -> usize { self.span }
//}

impl<T, S> Layer<T> for WithSpan<S>
where T: HasSpan + Into<S> {
    fn layered(t: T) -> Self {
        let span = t.span();
        WithSpan { wrapped: t.into(), span }
    }
}

impl<T> HasID for WithSpan<T>
    where T: HasID {
    fn id(&self) -> Option<ID> {
        self.deref().id()
    }
}


// =============================================================================
// === TO BE GENERATED =========================================================
// =============================================================================
// TODO: the definitions below should be removed and instead generated using
//  macros, as part of https://github.com/luna/enso/issues/338

// === AST ===

impl Ast {
    // TODO smart constructors for other cases
    //  as part of https://github.com/luna/enso/issues/338
    pub fn var(name: String) -> Ast {
        let var = Var{ name };
        Ast::from(var)
    }
}

// === Shape ===

// === Text Conversion Boilerplate ===
// support for transitive conversions, like:
// RawEscapeSth -> RawEscape -> SegmentRawEscape -> SegmentRaw

impl From<Unfinished> for SegmentRaw {
    fn from(value: Unfinished) -> Self {
        SegmentRawEscape{ code: value.into() }.into()
    }
}
impl From<Invalid> for SegmentRaw {
    fn from(value: Invalid) -> Self {
        SegmentRawEscape{ code: value.into() }.into()
    }
}
impl From<Slash> for SegmentRaw {
    fn from(value: Slash) -> Self {
        SegmentRawEscape{ code: value.into() }.into()
    }
}
impl From<Quote> for SegmentRaw {
    fn from(value: Quote) -> Self {
        SegmentRawEscape{ code: value.into() }.into()
    }
}
impl From<RawQuote> for SegmentRaw {
    fn from(value: RawQuote) -> Self {
        SegmentRawEscape{ code: value.into() }.into()
    }
}

// RawEscapeSth -> RawEscape -> SegmentRawEscape -> SegmentFmt
impl<T> From<Unfinished> for SegmentFmt<T> {
    fn from(value: Unfinished) -> Self {
        SegmentRawEscape{ code: value.into() }.into()
    }
}
impl<T> From<Invalid> for SegmentFmt<T> {
    fn from(value: Invalid) -> Self {
        SegmentRawEscape{ code: value.into() }.into()
    }
}
impl<T> From<Slash> for SegmentFmt<T> {
    fn from(value: Slash) -> Self {
        SegmentRawEscape{ code: value.into() }.into()
    }
}
impl<T> From<Quote> for SegmentFmt<T> {
    fn from(value: Quote) -> Self {
        SegmentRawEscape{ code: value.into() }.into()
    }
}
impl<T> From<RawQuote> for SegmentFmt<T> {
    fn from(value: RawQuote) -> Self {
        SegmentRawEscape{ code: value.into() }.into()
    }
}

impl<T> From<Escape> for SegmentFmt<T> {
    fn from(value: Escape) -> Self {
        SegmentEscape{ code: value.into() }.into()
    }
}

// =============
// === Tests ===
// =============

#[cfg(test)]
mod tests {
    use super::*;
    use serde::de::DeserializeOwned;

    /// Assert that given value round trips JSON serialization.
    fn round_trips<T>(input_val: &T)
    where T: Serialize + DeserializeOwned + PartialEq + Debug {
        let json_str            = serde_json::to_string(&input_val).unwrap();
        let deserialized_val: T = serde_json::from_str(&json_str).unwrap();
        assert_eq!(*input_val, deserialized_val);
    }

    #[test]
    fn var_smart_constructor() {
        let name = "foo".to_string();
        let v    = Ast::var(name.clone());
        match v.shape() {
            Shape::Var(var) if *var.name == name =>
                (),
            _ =>
                panic!("expected Var with name `{}`", name),
        }
    }

    #[test]
    fn ast_wrapping() {
        // We can convert `Var` into AST without worrying about span nor id.
        let sample_name = "foo".to_string();
        let v = Var{ name: sample_name.clone() };
        let ast = Ast::from(v);
        assert_eq!(ast.wrapped.id, None);
        assert_eq!(ast.wrapped.wrapped.span, sample_name.span());
    }

    #[test]
    fn serialization_round_trip() {
        let make_var = || Var { name: "foo".into() };
        round_trips(&make_var());

        let ast_without_id = Ast::new(make_var(), None);
        round_trips(&ast_without_id);

        let id        = Uuid::parse_str("15").ok();
        let ast_with_id = Ast::new(make_var(), id);
        round_trips(&ast_with_id);
    }

    #[test]
    fn deserialize_var() {
        let var_name = "foo";
        let uuid_str = "51e74fb9-75a4-499d-9ea3-a90a2663b4a1";

        let sample_json = serde_json::json!({
            "shape": { "Var":{"name": var_name}},
            "id": uuid_str,
            "span": var_name.len()
        });
        let sample_json_text = sample_json.to_string();
        let ast: Ast         = serde_json::from_str(&sample_json_text).unwrap();

        let expected_uuid = Uuid::parse_str(uuid_str).ok();
        assert_eq!(ast.id, expected_uuid);

        let expected_span = 3;
        assert_eq!(ast.span, expected_span);

        let expected_var   = Var { name: var_name.into() };
        let expected_shape = Shape::from(expected_var);
        assert_eq!(*ast.shape(), expected_shape);
    }
}

///////////////////////////////////

impl Blank {
    const REPR:char = '_';
}
impl Number {
    const BASE_SEPARATOR:char = '_';
}

/// Symbol enclosing raw Text line.
const RAW_QUOTE:char = '\'';

/// Symbol enclosing formatted Text line.
const FMT_QUOTE:char = '"';

/// Symbol used to break lines in Text block.
const NEWLINE:char = '\n';

/// Symbol introducing escape segment in the Text.
const BACKSLASH:char = '\\';

/// Symbol enclosing expression segment in the formatted Text.
const EXPR_QUOTE:char = '`';

/// Symbol that introduces UTF-16 code in the formatted Text segment.
const UNICODE16_INTRODUCER:char = 'u';

/// String that opens "UTF-21" code in the formatted Text segment.
const UNICODE21_OPENER:&str = "u{";

/// String that closese "UTF-21" code in the formatted Text segment.
const UNICODE21_CLOSER:&str = "}";

/// Symbol that introduces UTF-16 code in the formatted Text segment.
const UNICODE32_INTRODUCER:char = 'U';

impl TextBlockRaw {
    const QUOTE:&'static str = "\"\"\"";
}
impl<T> TextBlockFmt<T> {
    const QUOTE:&'static str = "'''";
}

///////////////////////////////////

/// Not an instance of `HasSpan`, as it needs to know parent block's offset.
impl<T: HasSpan> TextBlockLine<T> {
    fn span(&self, block_offset: usize) -> usize {
        let line_count              = self.empty_lines.len() + 1;
        let empty_lines_space:usize = self.empty_lines.iter().sum();
        let line_breaks             = line_count * NEWLINE.span();
        empty_lines_space + line_breaks + block_offset + self.text.span()
    }
}
///////////////////////////////////

impl<T: HasSpan> HasSpan for TextLine<T> {
    fn span(&self) -> usize {
        match self {
            TextLine::TextLineRaw(val) => val.span(),
            TextLine::TextLineFmt(val) => val.span(),
        }
    }
}

////

impl HasSpan for Empty {
    fn span(&self) -> usize {
        0
    }
}
impl HasSpan for Letter {
    fn span(&self) -> usize {
        self.char.span()
    }
}
impl HasSpan for Space {
    fn span(&self) -> usize {
        self.span
    }
}
impl HasSpan for Text {
    fn span(&self) -> usize {
        self.str.span()
    }
}
impl HasSpan for Seq {
    fn span(&self) -> usize {
        self.first.span() + self.second.span()
    }
}
// just dispatch
impl HasSpan for Builder {
    fn span(&self) -> usize {
        match self {
            Builder::Empty (val) => val.span(),
            Builder::Letter(val) => val.span(),
            Builder::Space (val) => val.span(),
            Builder::Text  (val) => val.span(),
            Builder::Seq   (val) => val.span(),
        }
    }
}

///////////////////////////////////////


// === RawEscape ===
impl HasSpan for Unfinished {
    fn span(&self) -> usize {
        0
    }
}
impl HasSpan for Invalid {
    fn span(&self) -> usize {
        self.str.span()
    }
}
impl HasSpan for Slash {
    fn span(&self) -> usize {
        1
    }
}
impl HasSpan for Quote {
        fn span(&self) -> usize {
        1
    }
}
impl HasSpan for RawQuote {
    fn span(&self) -> usize {
        1
    }
}

impl HasSpan for RawEscape {
    fn span(&self) -> usize {
        match self {
            RawEscape::Unfinished(val) => val.span(),
            RawEscape::Invalid   (val) => val.span(),
            RawEscape::Slash     (val) => val.span(),
            RawEscape::Quote     (val) => val.span(),
            RawEscape::RawQuote  (val) => val.span(),
        }
    }
}

///////////////////////////

impl HasSpan for SegmentPlain {
    fn span(&self) -> usize {
        self.value.span()
    }
}
impl HasSpan for SegmentRawEscape {
    fn span(&self) -> usize {
        self.code.span() + BACKSLASH.span()
    }
}
impl HasSpan for SegmentRaw {
    fn span(&self) -> usize {
        match self {
            SegmentRaw::SegmentPlain    (val) => val.span(),
            SegmentRaw::SegmentRawEscape(val) => val.span(),
        }
    }
}

////////////////////////////////////

impl<T: HasSpan> HasSpan for BlockLine<T> {
    fn span(&self) -> usize {
        self.elem.span() + self.off
    }
}

////////////////////////////////////

impl<T: HasSpan> HasSpan for SegmentExpr<T> {
    fn span(&self) -> usize {
        self.value.span() + 2 * EXPR_QUOTE.span()
    }
}
impl HasSpan for SegmentEscape {
    fn span(&self) -> usize {
        BACKSLASH.span() + self.code.span()
    }
}
impl<T: HasSpan> HasSpan for SegmentFmt<T> {
    fn span(&self) -> usize {
        match self {
            SegmentFmt::SegmentPlain    (val) => val.span(),
            SegmentFmt::SegmentRawEscape(val) => val.span(),
            SegmentFmt::SegmentExpr     (val) => val.span(),
            SegmentFmt::SegmentEscape   (val) => val.span(),
        }
    }
}

///////////////////////////////////
// escape

impl HasSpan for Escape {
    fn span(&self) -> usize {
        match self {
            Escape::Character{c              } => c.span(),
            Escape::Control  {name  , code: _} => name.span(),
            Escape::Number   {digits         } => digits.span(),
            Escape::Unicode16{digits         } =>
                UNICODE16_INTRODUCER.span() + digits.span(),
            Escape::Unicode21{digits} =>
                UNICODE21_OPENER.span() + digits.span()
                    + UNICODE21_CLOSER.span(),
            Escape::Unicode32{digits} =>
                UNICODE32_INTRODUCER.span() + digits.span(),
        }
    }
}

////////////////////////

impl HasSpan for Unrecognized {
    fn span(&self) -> usize {
        self.str.span()
    }
}
//impl<T> HasSpan for Unexpected<T> {
//    fn span(&self) -> usize {
//        self.str.len()
//    }
//}

impl HasSpan for InvalidQuote {
    fn span(&self) -> usize {
        self.quote.span()
    }
}
impl HasSpan for InlineBlock {
    fn span(&self) -> usize {
        self.quote.span()
    }
}
impl HasSpan for Blank {
    fn span(&self) -> usize {
        Blank::REPR.span()
    }
}
impl HasSpan for Var {
    fn span(&self) -> usize {
        self.name.span()
    }
}
impl HasSpan for Cons {
    fn span(&self) -> usize {
        self.name.span()
    }
}
impl HasSpan for Opr {
    fn span(&self) -> usize {
        self.name.span()
    }
}
impl HasSpan for Mod {
    fn span(&self) -> usize {
        self.name.span() + 1 // FIXME
    }
}
impl<T: HasSpan> HasSpan for InvalidSuffix<T> {
    fn span(&self) -> usize {
        self.elem.span() + self.suffix.span()
    }
}
impl HasSpan for Number {
    fn span(&self) -> usize {
        let base_span = match &self.base {
            Some(base) => base.span() + Number::BASE_SEPARATOR.span(),
            None       => 0,
        };
        base_span + self.int.span()
    }
}
impl HasSpan for DanglingBase {
    fn span(&self) -> usize {
        self.base.span() + Number::BASE_SEPARATOR.span()
    }
}
impl HasSpan for TextLineRaw {
    fn span(&self) -> usize {
        2 * RAW_QUOTE.span() + self.text.span()
    }
}
impl<T: HasSpan> HasSpan for TextLineFmt<T> {
    fn span(&self) -> usize {
        2 * FMT_QUOTE.span() + self.text.span()
    }
}
impl HasSpan for TextBlockRaw {
    fn span(&self) -> usize {
        let lines            =  self.text.iter();
        let line_spans       = lines.map(|line| line.span(self.offset));
        let lines_span:usize = line_spans.sum();
        TextBlockRaw::QUOTE.span() + self.spaces + lines_span
    }
}
impl<T: HasSpan> HasSpan for TextBlockFmt<T> {
    fn span(&self) -> usize {
        let lines            =  self.text.iter();
        let line_spans       = lines.map(|line| line.span(self.offset));
        let lines_span:usize = line_spans.sum();
        TextBlockFmt::<T>::QUOTE.span() + self.spaces + lines_span
    }
}
impl<T: HasSpan> HasSpan for TextUnclosed<T> {
    fn span(&self) -> usize {
        self.line.span() - 1 // FIXME
    }
}
impl<T: HasSpan> HasSpan for Prefix<T> {
    fn span(&self) -> usize {
        let func = self.func.span();
        let arg  = self.arg.span();
        func + self.off + arg
    }
}
impl<T: HasSpan> HasSpan for Infix<T> {
    fn span(&self) -> usize {
        self.larg.span() + self.loff + self.opr.span() + self.roff +
            self.rarg.span()
    }
}
impl<T: HasSpan> HasSpan for SectionLeft<T> {
    fn span(&self) -> usize {
        self.arg.span() + self.off + self.opr.span()
    }
}
impl<T: HasSpan> HasSpan for SectionRight<T> {
    fn span(&self) -> usize {
        self.opr.span() + self.off + self.arg.span()
    }
}
impl<T: HasSpan> HasSpan for SectionSides<T> {
    fn span(&self) -> usize {
        self.opr.span()
    }
}
impl<T: HasSpan> HasSpan for Module<T> {
    fn span(&self) -> usize {
        assert!(self.lines.len() > 0);
        let break_count = self.lines.len() - 1;
        let breaks_span = break_count * NEWLINE.span();
        let lines_span = self.lines.span();
        lines_span + breaks_span
    }
}
impl<T: HasSpan> HasSpan for Block<T> {
    fn span(&self) -> usize {
        let line_span = |line:&BlockLine<Option<T>>| {
            let indent = line.elem.as_ref().map_or(0, |_| self.indent);
            NEWLINE.span() + indent + line.span()
        };

        let head_span         = if self.is_orphan { 0 } else { 1 };
        let empty_lines       = self.empty_lines.iter();
        let empty_lines:usize = empty_lines.map(|line| line + 1).sum();
        let first_line        = self.indent + self.first_line.span();
        let lines      :usize = self.lines.iter().map(line_span).sum();
        head_span + empty_lines + first_line + lines
    }
}
//impl<T: HasSpan> HasSpan for Match<T> {
//    fn span(&self) -> usize {
//        self.pfx.span() + self.sets.span()
//    }
//}

impl<T: HasSpan> HasSpan for MacroPatternMatchRaw<T> {
    fn span(&self) -> usize {
        0
        //self.pfx.span() + self.sets.span()
    }
}


impl<T: HasSpan> HasSpan for Shape<T> {
    fn span(&self) -> usize {
        match self {
            // TODO: ? Shape::Unexpected
            Shape::Unrecognized (val) => val.span(),
            Shape::InvalidQuote (val) => val.span(),
            Shape::InlineBlock  (val) => val.span(),
            Shape::Blank        (val) => val.span(),
            Shape::Var          (val) => val.span(),
            Shape::Cons         (val) => val.span(),
            Shape::Opr          (val) => val.span(),
            Shape::Mod          (val) => val.span(),
            Shape::InvalidSuffix(val) => val.span(),
            Shape::Number       (val) => val.span(),
            Shape::DanglingBase (val) => val.span(),
            Shape::TextLineRaw  (val) => val.span(),
            Shape::TextLineFmt  (val) => val.span(),
            Shape::TextBlockRaw (val) => val.span(),
            Shape::TextBlockFmt (val) => val.span(),
            Shape::TextUnclosed (val) => val.span(),
            Shape::Prefix       (val) => val.span(),
            Shape::Infix        (val) => val.span(),
            Shape::SectionLeft  (val) => val.span(),
            Shape::SectionRight (val) => val.span(),
            Shape::SectionSides (val) => val.span(),
            Shape::Module       (val) => val.span(),
            Shape::Block        (val) => val.span(),
            _ => panic!("not implemented {}"),
        }
//        Match     { pfx      : Option<MacroPatternMatch<Shifted<Ast>>>
//            , segs     : ShiftedVec1<MacroMatchSegment<T>>
//            , resolved : Ast                                     },
//        Ambiguous { segs     : ShiftedVec1<MacroAmbiguousSegment>
//            , paths    : Tree<Ast, Unit>                         },
//
//        // === Spaceless AST ===
//        Comment   (Comment),
//        Import    (Import<T>),
//        Mixfix    (Mixfix<T>),
//        Group     (Group<T>),
//        Def       (Def<T>),
//        Foreign   (Foreign),
    }
}
