#[derive(Debug, Clone, PartialEq)]
pub enum PyLit {
    Int(String),
    Float(f64),
    Bool(bool),
    Nothing,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ENode {
    pub kind: EKind,
    pub line: usize,
}

#[derive(Debug, Clone, PartialEq)]
pub enum EKind {
    Lit(PyLit),
    StrLit(String),
    ListLit(Vec<ENode>),
    EmptyListE,
    Var(String),
    Field { obj: Box<ENode>, name: String },
    Bin { op: &'static str, l: Box<ENode>, r: Box<ENode> },
    NotE(Box<ENode>),
    Call { name: String, args: Vec<ENode> },
    ModuleCall { module: String, name: String, args: Vec<ENode> },
    NewThing { cls: String, setters: Vec<(String, ENode)> },
    NumVal(Box<ENode>),
    EverythingAfter { sep: Box<ENode>, e: Box<ENode> },
    CountOf(Box<ENode>),
    ItemAt { idx: Box<ENode>, e: Box<ENode> },
    FirstItem(Box<ENode>),
    LastItem(Box<ENode>),
    IsEmptyE(Box<ENode>),
    HasNoItems(Box<ENode>),
    ExistsE { e: Box<ENode>, flag: bool },
    IsNumberTest { e: Box<ENode>, negate: bool },
    RandomBetween { a: Box<ENode>, b: Box<ENode> },
    ContentsOf { e: Box<ENode>, as_json: bool },
    EveryTurnedInto { e: Box<ENode>, thing: String },
    CopyOf(Box<ENode>),
    AskE(Box<ENode>),
    QuestionE(Box<ENode>),
}

impl ENode {
    pub fn new(kind: EKind, line: usize) -> Self {
        ENode { kind, line }
    }

    pub fn boxed(kind: EKind, line: usize) -> Box<Self> {
        Box::new(ENode { kind, line })
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct SNode {
    pub kind: SKind,
    pub line: usize,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SBlock {
    pub stmts: Vec<SNode>,
    pub line: usize,
}

#[derive(Debug, Clone, PartialEq)]
pub enum SKind {
    Say { exprs: Vec<ENode>, newline: bool },
    Assign { target: ENode, expr: ENode },
    AddTo { name: String, expr: ENode },
    TakeFrom { name: String, expr: ENode },
    If { branches: Vec<(ENode, SBlock)>, otherwise: Option<SBlock> },
    RepeatForever { body: SBlock },
    RepeatUntil { cond: ENode, body: SBlock },
    RepeatWhile { cond: ENode, body: SBlock },
    RepeatTimes { count: ENode, body: SBlock },
    RepeatEach { var: String, iterable: ENode, body: SBlock },
    RepeatCounting { var: String, start: ENode, end: ENode, body: SBlock },
    BreakStmt,
    ContinueStmt,
    StopProgram,
    PauseProgram,
    Check { subject: ENode, arms: Vec<CheckArm>, otherwise: Option<SBlock> },
    TryStmt { body: SBlock, errname: Option<String>, handler: Option<SBlock> },
    FuncDef { name: String, params: Vec<String>, body: SBlock },
    ThingDef { name: String, fields: Vec<(String, Option<ENode>)> },
    ReturnStmt { expr: Option<ENode> },
    WaitStmt { amount: ENode, unit: &'static str },
    UseLib { text: String },
    UseModule { name: String, path: String },
    TrackStmt { name: String },
    UndoStmt { name: String, redo: bool },
    Contract { kind: &'static str, expr: ENode },
    RemoveStmt { expr: ENode },
    StoreJson { value: ENode, path: ENode },
    ExprStmt { expr: ENode },
    WhenProgramStarts { body: SBlock },
}

#[derive(Debug, Clone, PartialEq)]
pub struct CheckArm {
    pub kind: &'static str,
    pub val: Option<ENode>,
    pub neg: bool,
    pub body: SBlock,
}
