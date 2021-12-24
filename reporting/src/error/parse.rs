use roc_parse::parser::{ParseProblem, SyntaxError};
use roc_region::all::{Position, Region, LineInfo, LineColumn, LineColumnRegion};
use std::path::PathBuf;

use crate::report::{Report, RocDocAllocator, RocDocBuilder, Severity};
use ven_pretty::DocAllocator;

pub fn parse_problem<'a>(
    alloc: &'a RocDocAllocator<'a>,
    lines: &LineInfo,
    filename: PathBuf,
    _starting_line: u32,
    parse_problem: ParseProblem<SyntaxError<'a>>,
) -> Report<'a> {
    to_syntax_report(alloc, lines, filename, &parse_problem.problem, parse_problem.pos)
}

fn note_for_record_type_indent<'a>(alloc: &'a RocDocAllocator<'a>) -> RocDocBuilder<'a> {
    alloc.note("I may be confused by indentation")
}

fn note_for_record_pattern_indent<'a>(alloc: &'a RocDocAllocator<'a>) -> RocDocBuilder<'a> {
    alloc.note("I may be confused by indentation")
}

fn note_for_tag_union_type_indent<'a>(alloc: &'a RocDocAllocator<'a>) -> RocDocBuilder<'a> {
    alloc.note("I may be confused by indentation")
}

fn hint_for_tag_name<'a>(alloc: &'a RocDocAllocator<'a>) -> RocDocBuilder<'a> {
    alloc.concat(vec![
        alloc.hint("Tag names "),
        alloc.reflow("start with an uppercase letter, like "),
        alloc.parser_suggestion("Err"),
        alloc.text(" or "),
        alloc.parser_suggestion("Green"),
        alloc.text("."),
    ])
}

fn hint_for_private_tag_name<'a>(alloc: &'a RocDocAllocator<'a>) -> RocDocBuilder<'a> {
    alloc.concat(vec![
        alloc.hint("Private tag names "),
        alloc.reflow("start with an `@` symbol followed by an uppercase letter, like "),
        alloc.parser_suggestion("@UID"),
        alloc.text(" or "),
        alloc.parser_suggestion("@SecretKey"),
        alloc.text("."),
    ])
}

fn record_patterns_look_like<'a>(alloc: &'a RocDocAllocator<'a>) -> RocDocBuilder<'a> {
    alloc.concat(vec![
        alloc.reflow(r"Record pattern look like "),
        alloc.parser_suggestion("{ name, age: currentAge },"),
        alloc.reflow(" so I was expecting to see a field name next."),
    ])
}

fn to_syntax_report<'a>(
    alloc: &'a RocDocAllocator<'a>,
    lines: &LineInfo,
    filename: PathBuf,
    parse_problem: &roc_parse::parser::SyntaxError<'a>,
    start: Position,
) -> Report<'a> {
    use SyntaxError::*;

    let report = |doc| Report {
        filename: filename.clone(),
        doc,
        title: "PARSE PROBLEM".to_string(),
        severity: Severity::RuntimeError,
    };

    let region = Region::from_pos(start);

    match parse_problem {
        SyntaxError::ConditionFailed => {
            let doc = alloc.stack(vec![
                alloc.reflow("A condition failed:"),
                alloc.region(lines.convert_region(region)),
            ]);

            Report {
                filename,
                doc,
                title: "PARSE PROBLEM".to_string(),
                severity: Severity::RuntimeError,
            }
        }
        SyntaxError::ArgumentsBeforeEquals(region) => {
            let doc = alloc.stack(vec![
                alloc.reflow("Unexpected tokens in front of the `=` symbol:"),
                alloc.region(lines.convert_region(*region)),
            ]);

            Report {
                filename,
                doc,
                title: "PARSE PROBLEM".to_string(),
                severity: Severity::RuntimeError,
            }
        }
        Unexpected(region) => {
            let mut region = lines.convert_region(*region);
            if region.start().column == region.end().column {
                region = LineColumnRegion::new(region.start(), region.end().bump_column(1));
            }

            let doc = alloc.stack(vec![
                alloc.concat(vec![
                    alloc.reflow("Unexpected token "),
                    // context(alloc, &parse_problem.context_stack, "here"),
                    alloc.text(":"),
                ]),
                alloc.region(region),
            ]);

            report(doc)
        }
        NotEndOfFile(pos) => {
            let surroundings = Region::new(start, *pos);
            let region = Region::from_pos(*pos);

            let doc = alloc.stack(vec![
                alloc.reflow(r"I expected to reach the end of the file, but got stuck here:"),
                alloc.region_with_subregion(lines.convert_region(surroundings), lines.convert_region(region)),
                alloc.concat(vec![alloc.reflow("no hints")]),
            ]);

            Report {
                filename,
                doc,
                title: "NOT END OF FILE".to_string(),
                severity: Severity::RuntimeError,
            }
        }
        SyntaxError::Eof(region) => {
            let doc = alloc.stack(vec![alloc.reflow("End of Field"), alloc.region(lines.convert_region(*region))]);

            Report {
                filename,
                doc,
                title: "PARSE PROBLEM".to_string(),
                severity: Severity::RuntimeError,
            }
        }
        SyntaxError::OutdentedTooFar => {
            let doc = alloc.stack(vec![alloc.reflow("OutdentedTooFar")]);

            Report {
                filename,
                doc,
                title: "PARSE PROBLEM".to_string(),
                severity: Severity::RuntimeError,
            }
        }
        Type(typ) => to_type_report(alloc, lines, filename, typ, Position::default()),
        Pattern(pat) => to_pattern_report(alloc, lines, filename, pat, Position::default()),
        Expr(expr) => to_expr_report(
            alloc,
            lines,
            filename,
            Context::InDef(start),
            expr,
            Position::default(),
        ),
        Header(header) => to_header_report(alloc, lines, filename, header, Position::default()),
        _ => todo!("unhandled parse error: {:?}", parse_problem),
    }
}

#[allow(clippy::enum_variant_names)]
enum Context {
    InNode(Node, Position, Box<Context>),
    InDef(Position),
    InDefFinalExpr(Position),
}

enum Node {
    WhenCondition,
    WhenBranch,
    WhenIfGuard,
    IfCondition,
    IfThenBranch,
    IfElseBranch,
    ListElement,
    InsideParens,
    RecordConditionalDefault,
    StringFormat,
}

fn to_expr_report<'a>(
    alloc: &'a RocDocAllocator<'a>,
    lines: &LineInfo,
    filename: PathBuf,
    context: Context,
    parse_problem: &roc_parse::parser::EExpr<'a>,
    start: Position,
) -> Report<'a> {
    use roc_parse::parser::EExpr;

    match parse_problem {
        EExpr::If(if_, pos) => to_if_report(alloc, lines, filename, context, if_, *pos),
        EExpr::When(when, pos) => to_when_report(alloc, lines, filename, context, when, *pos),
        EExpr::Lambda(lambda, pos) => to_lambda_report(alloc, lines, filename, context, lambda, *pos),
        EExpr::List(list, pos) => to_list_report(alloc, lines, filename, context, list, *pos),
        EExpr::Str(string, pos) => to_str_report(alloc, lines, filename, context, string, *pos),
        EExpr::InParens(expr, pos) => {
            to_expr_in_parens_report(alloc, lines, filename, context, expr, *pos)
        }
        EExpr::Type(tipe, pos) => to_type_report(alloc, lines, filename, tipe, *pos),
        EExpr::ElmStyleFunction(region, pos) => {
            let surroundings = Region::new(start, *pos);
            let region = *region;

            let doc = alloc.stack(vec![
                alloc.reflow(r"I am partway through parsing a definition, but I got stuck here:"),
                alloc.region_with_subregion(lines.convert_region(surroundings), lines.convert_region(region)),
                alloc.concat(vec![
                    alloc.reflow("Looks like you are trying to define a function. "),
                    alloc.reflow("In roc, functions are always written as a lambda, like "),
                    alloc.parser_suggestion("increment = \\n -> n + 1"),
                    alloc.reflow("."),
                ]),
            ]);

            Report {
                filename,
                doc,
                title: "ARGUMENTS BEFORE EQUALS".to_string(),
                severity: Severity::RuntimeError,
            }
        }

        EExpr::BadOperator(op, pos) => {
            let surroundings = Region::new(start, *pos);
            let region = Region::new(*pos, pos.bump_column(op.len() as u16));

            let suggestion = match *op {
                "|" => vec![
                    alloc.reflow("Maybe you want "),
                    alloc.parser_suggestion("||"),
                    alloc.reflow(" or "),
                    alloc.parser_suggestion("|>"),
                    alloc.reflow(" instead?"),
                ],
                "++" => vec![
                    alloc.reflow("To concatenate two lists or strings, try using "),
                    alloc.parser_suggestion("List.concat"),
                    alloc.reflow(" or "),
                    alloc.parser_suggestion("Str.concat"),
                    alloc.reflow(" instead."),
                ],
                ":" => vec![alloc.stack(vec![
                    alloc.concat(vec![
                        alloc.reflow("The has-type operator "),
                        alloc.parser_suggestion(":"),
                        alloc.reflow(" can only occur in a definition's type signature, like"),
                    ]),
                    alloc
                        .vcat(vec![
                            alloc.text("increment : I64 -> I64"),
                            alloc.text("increment = \\x -> x + 1"),
                        ])
                        .indent(4),
                ])],
                "->" => match context {
                    Context::InNode(Node::WhenBranch, _pos, _) => {
                        return to_unexpected_arrow_report(alloc, lines, filename, *pos, start);
                    }
                    _ => {
                        vec![alloc.stack(vec![
                            alloc.concat(vec![
                                alloc.reflow("The arrow "),
                                alloc.parser_suggestion("->"),
                                alloc.reflow(" is only used to define cases in a "),
                                alloc.keyword("when"),
                                alloc.reflow("."),
                            ]),
                            alloc
                                .vcat(vec![
                                    alloc.text("when color is"),
                                    alloc.text("Red -> \"stop!\"").indent(4),
                                    alloc.text("Green -> \"go!\"").indent(4),
                                ])
                                .indent(4),
                        ])]
                    }
                },
                "!" => vec![
                    alloc.reflow("The boolean negation operator "),
                    alloc.parser_suggestion("!"),
                    alloc.reflow(" must occur immediately before an expression, like "),
                    alloc.parser_suggestion("!(List.isEmpty primes)"),
                    alloc.reflow(". There cannot be a space between the "),
                    alloc.parser_suggestion("!"),
                    alloc.reflow(" and the expression after it."),
                ],
                _ => vec![
                    alloc.reflow("I have no specific suggestion for this operator, "),
                    alloc.reflow("see TODO for the full list of operators in Roc."),
                ],
            };

            let doc = alloc.stack(vec![
                alloc.reflow(r"This looks like an operator, but it's not one I recognize!"),
                alloc.region_with_subregion(lines.convert_region(surroundings), lines.convert_region(region)),
                alloc.concat(suggestion),
            ]);

            Report {
                filename,
                doc,
                title: "UNKNOWN OPERATOR".to_string(),
                severity: Severity::RuntimeError,
            }
        }

        EExpr::Ident(_pos) => unreachable!("another branch would be taken"),

        EExpr::QualifiedTag(pos) => {
            let surroundings = Region::new(start, *pos);
            let region = Region::from_pos(*pos);

            let doc = alloc.stack(vec![
                alloc.reflow(r"I am very confused by this identifier:"),
                alloc.region_with_subregion(lines.convert_region(surroundings), lines.convert_region(region)),
                alloc.concat(vec![
                    alloc.reflow("Are you trying to qualify a name? I am execting something like "),
                    alloc.parser_suggestion("Json.Decode.string"),
                    alloc.reflow(". Maybe you are trying to qualify a tag? Tags like "),
                    alloc.parser_suggestion("Err"),
                    alloc.reflow(" are globally scoped in roc, and cannot be qualified."),
                ]),
            ]);

            Report {
                filename,
                doc,
                title: "WEIRD IDENTIFIER".to_string(),
                severity: Severity::RuntimeError,
            }
        }

        EExpr::Start(pos) | EExpr::IndentStart(pos) => {
            let (title, expecting) = match &context {
                Context::InNode { .. } | Context::InDef { .. } => (
                    "MISSING EXPRESSION",
                    alloc.concat(vec![
                        alloc.reflow("I was expecting to see an expression like "),
                        alloc.parser_suggestion("42"),
                        alloc.reflow(" or "),
                        alloc.parser_suggestion("\"hello\""),
                        alloc.text("."),
                    ]),
                ),
                Context::InDefFinalExpr { .. } => (
                    "MISSING FINAL EXPRESSION",
                    alloc.stack(vec![
                        alloc.concat(vec![
                            alloc.reflow("This definition is missing a final expression."),
                            alloc.reflow(" A nested definition must be followed by"),
                            alloc.reflow(" either another definition, or an expression"),
                        ]),
                        alloc.vcat(vec![
                            alloc.text("x = 4").indent(4),
                            alloc.text("y = 2").indent(4),
                            alloc.text(""),
                            alloc.text("x + y").indent(4),
                        ]),
                    ]),
                ),
            };

            let (context_pos, a_thing) = match context {
                Context::InNode(node, pos, _) => match node {
                    Node::WhenCondition | Node::WhenBranch | Node::WhenIfGuard => (
                        pos,
                        alloc.concat(vec![
                            alloc.text("an "),
                            alloc.keyword("when"),
                            alloc.text(" expression"),
                        ]),
                    ),
                    Node::IfCondition | Node::IfThenBranch | Node::IfElseBranch => (
                        pos,
                        alloc.concat(vec![
                            alloc.text("an "),
                            alloc.keyword("if"),
                            alloc.text(" expression"),
                        ]),
                    ),
                    Node::ListElement => (pos, alloc.text("a list")),
                    Node::RecordConditionalDefault => (pos, alloc.text("record field default")),
                    Node::StringFormat => (pos, alloc.text("a string format")),
                    Node::InsideParens => (pos, alloc.text("some parentheses")),
                },
                Context::InDef(pos) => (pos, alloc.text("a definition")),
                Context::InDefFinalExpr(pos) => {
                    (pos, alloc.text("a definition's final expression"))
                }
            };

            let surroundings = Region::new(context_pos, *pos);
            let region = Region::from_pos(*pos);

            let doc = alloc.stack(vec![
                alloc.concat(vec![
                    alloc.reflow(r"I am partway through parsing "),
                    a_thing,
                    alloc.reflow(", but I got stuck here:"),
                ]),
                alloc.region_with_subregion(lines.convert_region(surroundings), lines.convert_region(region)),
                expecting,
            ]);

            Report {
                filename,
                doc,
                title: title.to_string(),
                severity: Severity::RuntimeError,
            }
        }

        EExpr::DefMissingFinalExpr(pos) => {
            let surroundings = Region::new(start, *pos);
            let region = Region::from_pos(*pos);

            let doc = alloc.stack(vec![
                alloc.reflow(r"I am partway through parsing a definition, but I got stuck here:"),
                alloc.region_with_subregion(lines.convert_region(surroundings), lines.convert_region(region)),
                alloc.concat(vec![
                    alloc.reflow("This definition is missing a final expression."),
                    alloc.reflow(" A nested definition must be followed by"),
                    alloc.reflow(" either another definition, or an expression"),
                ]),
                alloc.vcat(vec![
                    alloc.text("x = 4").indent(4),
                    alloc.text("y = 2").indent(4),
                    alloc.text(""),
                    alloc.text("x + y").indent(4),
                ]),
            ]);

            Report {
                filename,
                doc,
                title: "MISSING FINAL EXPRESSION".to_string(),
                severity: Severity::RuntimeError,
            }
        }

        EExpr::DefMissingFinalExpr2(expr, pos) => {
            to_expr_report(alloc, lines, filename, Context::InDefFinalExpr(start), expr, *pos)
        }

        EExpr::BadExprEnd(pos) => {
            let surroundings = Region::new(start, *pos);
            let region = Region::from_pos(*pos);

            let doc = alloc.stack(vec![
                alloc.reflow(r"I got stuck here:"),
                alloc.region_with_subregion(lines.convert_region(surroundings), lines.convert_region(region)),
                alloc.concat(vec![
                    alloc.reflow("Whatever I am running into is confusing me a lot! "),
                    alloc.reflow("Normally I can give fairly specific hints, "),
                    alloc.reflow("but something is really tripping me up this time."),
                ]),
            ]);

            Report {
                filename,
                doc,
                title: "SYNTAX PROBLEM".to_string(),
                severity: Severity::RuntimeError,
            }
        }

        EExpr::Colon(pos) => {
            let surroundings = Region::new(start, *pos);
            let region = Region::from_pos(*pos);

            let doc = alloc.stack(vec![
                alloc.reflow(r"I am partway through parsing a definition, but I got stuck here:"),
                alloc.region_with_subregion(lines.convert_region(surroundings), lines.convert_region(region)),
                alloc.concat(vec![
                    alloc.reflow("Looks like you are trying to define a function. "),
                    alloc.reflow("In roc, functions are always written as a lambda, like "),
                    alloc.parser_suggestion("increment = \\n -> n + 1"),
                    alloc.reflow("."),
                ]),
            ]);

            Report {
                filename,
                doc,
                title: "ARGUMENTS BEFORE EQUALS".to_string(),
                severity: Severity::RuntimeError,
            }
        }

        EExpr::BackpassArrow(pos) => {
            let surroundings = Region::new(start, *pos);
            let region = Region::from_pos(*pos);

            let doc = alloc.stack(vec![
                alloc.reflow(r"I am partway through parsing an expression, but I got stuck here:"),
                alloc.region_with_subregion(lines.convert_region(surroundings), lines.convert_region(region)),
                alloc.concat(vec![
                    alloc.reflow("Looks like you are trying to define a function. ")
                ]),
            ]);

            Report {
                filename,
                doc,
                title: "BAD BACKPASSING ARROW".to_string(),
                severity: Severity::RuntimeError,
            }
        }

        EExpr::Space(error, pos) => to_space_report(alloc, lines, filename, error, *pos),

        _ => todo!("unhandled parse error: {:?}", parse_problem),
    }
}

fn to_lambda_report<'a>(
    alloc: &'a RocDocAllocator<'a>,
    lines: &LineInfo,
    filename: PathBuf,
    _context: Context,
    parse_problem: &roc_parse::parser::ELambda<'a>,
    start: Position,
) -> Report<'a> {
    use roc_parse::parser::ELambda;

    match *parse_problem {
        ELambda::Arrow(pos) => match what_is_next(alloc.src_lines, lines.convert_pos(pos)) {
            Next::Token("=>") => {
                let surroundings = Region::new(start, pos);
                let region = Region::from_pos(pos);

                let doc = alloc.stack(vec![
                    alloc
                        .reflow(r"I am partway through parsing a function argument list, but I got stuck here:"),
                    alloc.region_with_subregion(lines.convert_region(surroundings), lines.convert_region(region)),
                    alloc.concat(vec![
                        alloc.reflow("I was expecting a "),
                        alloc.parser_suggestion("->"),
                        alloc.reflow(" next."),
                    ]),
                ]);

                Report {
                    filename,
                    doc,
                    title: "WEIRD ARROW".to_string(),
                    severity: Severity::RuntimeError,
                }
            }
            _ => {
                let surroundings = Region::new(start, pos);
                let region = Region::from_pos(pos);

                let doc = alloc.stack(vec![
                    alloc
                        .reflow(r"I am partway through parsing a function argument list, but I got stuck here:"),
                    alloc.region_with_subregion(lines.convert_region(surroundings), lines.convert_region(region)),
                    alloc.concat(vec![
                        alloc.reflow("I was expecting a "),
                        alloc.parser_suggestion("->"),
                        alloc.reflow(" next."),
                    ]),
                ]);

                Report {
                    filename,
                    doc,
                    title: "MISSING ARROW".to_string(),
                    severity: Severity::RuntimeError,
                }
            }
        },

        ELambda::Comma(pos) => match what_is_next(alloc.src_lines, lines.convert_pos(pos)) {
            Next::Token("=>") => {
                let surroundings = Region::new(start, pos);
                let region = Region::from_pos(pos);

                let doc = alloc.stack(vec![
                    alloc
                        .reflow(r"I am partway through parsing a function argument list, but I got stuck here:"),
                    alloc.region_with_subregion(lines.convert_region(surroundings), lines.convert_region(region)),
                    alloc.concat(vec![
                        alloc.reflow("I was expecting a "),
                        alloc.parser_suggestion("->"),
                        alloc.reflow(" next."),
                    ]),
                ]);

                Report {
                    filename,
                    doc,
                    title: "WEIRD ARROW".to_string(),
                    severity: Severity::RuntimeError,
                }
            }
            _ => {
                let surroundings = Region::new(start, pos);
                let region = Region::from_pos(pos);

                let doc = alloc.stack(vec![
                    alloc
                        .reflow(r"I am partway through parsing a function argument list, but I got stuck here:"),
                    alloc.region_with_subregion(lines.convert_region(surroundings), lines.convert_region(region)),
                    alloc.concat(vec![
                        alloc.reflow("I was expecting a "),
                        alloc.parser_suggestion("->"),
                        alloc.reflow(" next."),
                    ]),
                ]);

                Report {
                    filename,
                    doc,
                    title: "MISSING ARROW".to_string(),
                    severity: Severity::RuntimeError,
                }
            }
        },

        ELambda::Arg(pos) => match what_is_next(alloc.src_lines, lines.convert_pos(pos)) {
            Next::Other(Some(',')) => {
                let surroundings = Region::new(start, pos);
                let region = Region::from_pos(pos);

                let doc = alloc.stack(vec![
                    alloc
                        .reflow(r"I am partway through parsing a function argument list, but I got stuck at this comma:"),
                    alloc.region_with_subregion(lines.convert_region(surroundings), lines.convert_region(region)),
                    alloc.concat(vec![
                        alloc.reflow("I was expecting an argument pattern before this, "),
                        alloc.reflow("so try adding an argument before the comma and see if that helps?"),
                    ]),
                ]);

                Report {
                    filename,
                    doc,
                    title: "UNFINISHED ARGUMENT LIST".to_string(),
                    severity: Severity::RuntimeError,
                }
            }
            _ => {
                let surroundings = Region::new(start, pos);
                let region = Region::from_pos(pos);

                let doc = alloc.stack(vec![
                    alloc
                        .reflow(r"I am partway through parsing a function argument list, but I got stuck here:"),
                    alloc.region_with_subregion(lines.convert_region(surroundings), lines.convert_region(region)),
                    alloc.concat(vec![
                        alloc.reflow("I was expecting an argument pattern before this, "),
                        alloc.reflow("so try adding an argument and see if that helps?"),
                    ]),
                ]);

                Report {
                    filename,
                    doc,
                    title: "MISSING ARROW".to_string(),
                    severity: Severity::RuntimeError,
                }
            }
        },

        ELambda::Start(_pos) => unreachable!("another branch would have been taken"),

        ELambda::Body(expr, pos) => {
            to_expr_report(alloc, lines, filename, Context::InDef(start), expr, pos)
        }
        ELambda::Pattern(ref pattern, pos) => to_pattern_report(alloc, lines, filename, pattern, pos),
        ELambda::Space(error, pos) => to_space_report(alloc, lines, filename, &error, pos),

        ELambda::IndentArrow(pos) => to_unfinished_lambda_report(
            alloc,
            lines,
            filename,
            pos,
            start,
            alloc.concat(vec![
                alloc.reflow(r"I just saw a pattern, so I was expecting to see a "),
                alloc.parser_suggestion("->"),
                alloc.reflow(" next."),
            ]),
        ),

        ELambda::IndentBody(pos) => to_unfinished_lambda_report(
            alloc,
            lines,
            filename,
            pos,
            start,
            alloc.concat(vec![
                alloc.reflow(r"I just saw a pattern, so I was expecting to see a "),
                alloc.parser_suggestion("->"),
                alloc.reflow(" next."),
            ]),
        ),

        ELambda::IndentArg(pos) => to_unfinished_lambda_report(
            alloc,
            lines,
            filename,
            pos,
            start,
            alloc.concat(vec![
                alloc.reflow(r"I just saw a pattern, so I was expecting to see a "),
                alloc.parser_suggestion("->"),
                alloc.reflow(" next."),
                alloc.reflow(r"I was expecting to see a expression next"),
            ]),
        ),
    }
}

fn to_unfinished_lambda_report<'a>(
    alloc: &'a RocDocAllocator<'a>,
    lines: &LineInfo,
    filename: PathBuf,
    pos: Position,
    start: Position,
    message: RocDocBuilder<'a>,
) -> Report<'a> {
    let surroundings = Region::new(start, pos);
    let region = Region::from_pos(pos);

    let doc = alloc.stack(vec![
        alloc.concat(vec![
            alloc.reflow(r"I was partway through parsing a "),
            alloc.reflow(r" function, but I got stuck here:"),
        ]),
        alloc.region_with_subregion(lines.convert_region(surroundings), lines.convert_region(region)),
        message,
    ]);

    Report {
        filename,
        doc,
        title: "UNFINISHED FUNCTION".to_string(),
        severity: Severity::RuntimeError,
    }
}

fn to_str_report<'a>(
    alloc: &'a RocDocAllocator<'a>,
    lines: &LineInfo,
    filename: PathBuf,
    context: Context,
    parse_problem: &roc_parse::parser::EString<'a>,
    start: Position,
) -> Report<'a> {
    use roc_parse::parser::EString;

    match *parse_problem {
        EString::Open(_pos) => unreachable!("another branch would be taken"),
        EString::Format(expr, pos) => to_expr_report(
            alloc,
            lines,
            filename,
            Context::InNode(Node::StringFormat, start, Box::new(context)),
            expr,
            pos,
        ),
        EString::Space(error, pos) => to_space_report(alloc, lines, filename, &error, pos),
        EString::UnknownEscape(pos) => {
            let surroundings = Region::new(start, pos);
            let region = Region::new(pos, pos.bump_column(2));

            let suggestion = |msg, sugg| {
                alloc
                    .text("- ")
                    .append(alloc.reflow(msg))
                    .append(alloc.parser_suggestion(sugg))
            };

            let doc = alloc.stack(vec![
                alloc.concat(vec![
                    alloc.reflow(r"I was partway through parsing a "),
                    alloc.reflow(r" string literal, but I got stuck here:"),
                ]),
                alloc.region_with_subregion(lines.convert_region(surroundings), lines.convert_region(region)),
                alloc.concat(vec![
                    alloc.reflow(r"This is not an escape sequence I recognize."),
                    alloc.reflow(r" After a backslash, I am looking for one of these:"),
                ]),
                alloc
                    .vcat(vec![
                        suggestion("A newline: ", "\\n"),
                        suggestion("A caret return: ", "\\r"),
                        suggestion("A tab: ", "\\t"),
                        suggestion("An escaped quote: ", "\\\""),
                        suggestion("An escaped backslash: ", "\\\\"),
                        suggestion("A unicode code point: ", "\\u(00FF)"),
                        suggestion("An interpolated string: ", "\\(myVariable)"),
                    ])
                    .indent(4),
            ]);

            Report {
                filename,
                doc,
                title: "WEIRD ESCAPE".to_string(),
                severity: Severity::RuntimeError,
            }
        }
        EString::CodePtOpen(pos) | EString::CodePtEnd(pos) => {
            let surroundings = Region::new(start, pos);
            let region = Region::from_pos(pos);

            let doc = alloc.stack(vec![
                alloc.reflow(
                    r"I am partway through parsing a unicode code point, but I got stuck here:",
                ),
                alloc.region_with_subregion(lines.convert_region(surroundings), lines.convert_region(region)),
                alloc.concat(vec![
                    alloc.reflow(r"I was expecting a hexadecimal number, like "),
                    alloc.parser_suggestion("\\u(1100)"),
                    alloc.reflow(" or "),
                    alloc.parser_suggestion("\\u(00FF)"),
                    alloc.text("."),
                ]),
                alloc.reflow(r"Learn more about working with unicode in roc at TODO"),
            ]);

            Report {
                filename,
                doc,
                title: "WEIRD CODE POINT".to_string(),
                severity: Severity::RuntimeError,
            }
        }
        EString::FormatEnd(pos) => {
            let surroundings = Region::new(start, pos);
            let region = Region::from_pos(pos);

            let doc = alloc.stack(vec![
                alloc.reflow(r"I cannot find the end of this format expression:"),
                alloc.region_with_subregion(lines.convert_region(surroundings), lines.convert_region(region)),
                alloc.concat(vec![
                    alloc.reflow(r"You could change it to something like "),
                    alloc.parser_suggestion("\"The count is \\(count\\)\""),
                    alloc.reflow("."),
                ]),
            ]);

            Report {
                filename,
                doc,
                title: "ENDLESS FORMAT".to_string(),
                severity: Severity::RuntimeError,
            }
        }
        EString::EndlessSingle(pos) => {
            let surroundings = Region::new(start, pos);
            let region = Region::from_pos(pos);

            let doc = alloc.stack(vec![
                alloc.reflow(r"I cannot find the end of this string:"),
                alloc.region_with_subregion(lines.convert_region(surroundings), lines.convert_region(region)),
                alloc.concat(vec![
                    alloc.reflow(r"You could change it to something like "),
                    alloc.parser_suggestion("\"to be or not to be\""),
                    alloc.reflow(" or even just "),
                    alloc.parser_suggestion("\"\""),
                    alloc.reflow("."),
                ]),
            ]);

            Report {
                filename,
                doc,
                title: "ENDLESS STRING".to_string(),
                severity: Severity::RuntimeError,
            }
        }
        EString::EndlessMulti(pos) => {
            let surroundings = Region::new(start, pos);
            let region = Region::from_pos(pos);

            let doc = alloc.stack(vec![
                alloc.reflow(r"I cannot find the end of this block string:"),
                alloc.region_with_subregion(lines.convert_region(surroundings), lines.convert_region(region)),
                alloc.concat(vec![
                    alloc.reflow(r"You could change it to something like "),
                    alloc.parser_suggestion("\"\"\"to be or not to be\"\"\""),
                    alloc.reflow(" or even just "),
                    alloc.parser_suggestion("\"\"\"\"\"\""),
                    alloc.reflow("."),
                ]),
            ]);

            Report {
                filename,
                doc,
                title: "ENDLESS STRING".to_string(),
                severity: Severity::RuntimeError,
            }
        }
    }
}
fn to_expr_in_parens_report<'a>(
    alloc: &'a RocDocAllocator<'a>,
    lines: &LineInfo,
    filename: PathBuf,
    context: Context,
    parse_problem: &roc_parse::parser::EInParens<'a>,
    start: Position,
) -> Report<'a> {
    use roc_parse::parser::EInParens;

    match *parse_problem {
        EInParens::Space(error, pos) => to_space_report(alloc, lines, filename, &error, pos),
        EInParens::Expr(expr, pos) => to_expr_report(
            alloc,
            lines,
            filename,
            Context::InNode(Node::InsideParens, start, Box::new(context)),
            expr,
            pos,
        ),
        EInParens::End(pos) | EInParens::IndentEnd(pos) => {
            let surroundings = Region::new(start, pos);
            let region = Region::from_pos(pos);

            let doc = alloc.stack(vec![
                alloc
                    .reflow("I am partway through parsing a record pattern, but I got stuck here:"),
                alloc.region_with_subregion(lines.convert_region(surroundings), lines.convert_region(region)),
                alloc.concat(vec![
                    alloc.reflow(
                        r"I was expecting to see a closing parenthesis next, so try adding a ",
                    ),
                    alloc.parser_suggestion(")"),
                    alloc.reflow(" and see if that helps?"),
                ]),
            ]);

            Report {
                filename,
                doc,
                title: "UNFINISHED PARENTHESES".to_string(),
                severity: Severity::RuntimeError,
            }
        }
        EInParens::Open(pos) | EInParens::IndentOpen(pos) => {
            let surroundings = Region::new(start, pos);
            let region = Region::from_pos(pos);

            let doc = alloc.stack(vec![
                alloc.reflow(
                    r"I just started parsing an expression in parentheses, but I got stuck here:",
                ),
                alloc.region_with_subregion(lines.convert_region(surroundings), lines.convert_region(region)),
                alloc.concat(vec![
                    alloc.reflow(r"An expression in parentheses looks like "),
                    alloc.parser_suggestion("(32)"),
                    alloc.reflow(r" or "),
                    alloc.parser_suggestion("(\"hello\")"),
                    alloc.reflow(" so I was expecting to see an expression next."),
                ]),
            ]);

            Report {
                filename,
                doc,
                title: "UNFINISHED PARENTHESES".to_string(),
                severity: Severity::RuntimeError,
            }
        }
    }
}

fn to_list_report<'a>(
    alloc: &'a RocDocAllocator<'a>,
    lines: &LineInfo,
    filename: PathBuf,
    context: Context,
    parse_problem: &roc_parse::parser::EList<'a>,
    start: Position,
) -> Report<'a> {
    use roc_parse::parser::EList;

    match *parse_problem {
        EList::Space(error, pos) => to_space_report(alloc, lines, filename, &error, pos),

        EList::Expr(expr, pos) => to_expr_report(
            alloc,
            lines,
            filename,
            Context::InNode(Node::ListElement, start, Box::new(context)),
            expr,
            pos,
        ),

        EList::Open(pos) | EList::End(pos) => match what_is_next(alloc.src_lines, lines.convert_pos(pos)) {
            Next::Other(Some(',')) => {
                let surroundings = Region::new(start, pos);
                let region = Region::from_pos(pos);

                let doc = alloc.stack(vec![
                    alloc.reflow(
                        r"I am partway through started parsing a list, but I got stuck here:",
                    ),
                    alloc.region_with_subregion(lines.convert_region(surroundings), lines.convert_region(region)),
                    alloc.concat(vec![
                        alloc.reflow(r"I was expecting to see a list entry before this comma, "),
                        alloc.reflow(r"so try adding a list entry"),
                        alloc.reflow(r" and see if that helps?"),
                    ]),
                ]);
                Report {
                    filename,
                    doc,
                    title: "UNFINISHED LIST".to_string(),
                    severity: Severity::RuntimeError,
                }
            }
            _ => {
                let surroundings = Region::new(start, pos);
                let region = Region::from_pos(pos);

                let doc = alloc.stack(vec![
                    alloc.reflow(
                        r"I am partway through started parsing a list, but I got stuck here:",
                    ),
                    alloc.region_with_subregion(lines.convert_region(surroundings), lines.convert_region(region)),
                    alloc.concat(vec![
                        alloc.reflow(
                            r"I was expecting to see a closing square bracket before this, ",
                        ),
                        alloc.reflow(r"so try adding a "),
                        alloc.parser_suggestion("]"),
                        alloc.reflow(r" and see if that helps?"),
                    ]),
                    alloc.concat(vec![
                        alloc.note("When "),
                        alloc.reflow(r"I get stuck like this, "),
                        alloc.reflow(r"it usually means that there is a missing parenthesis "),
                        alloc.reflow(r"or bracket somewhere earlier. "),
                        alloc.reflow(r"It could also be a stray keyword or operator."),
                    ]),
                ]);

                Report {
                    filename,
                    doc,
                    title: "UNFINISHED LIST".to_string(),
                    severity: Severity::RuntimeError,
                }
            }
        },

        EList::IndentOpen(pos) | EList::IndentEnd(pos) => {
            let surroundings = Region::new(start, pos);
            let region = Region::from_pos(pos);

            let doc = alloc.stack(vec![
                alloc.reflow(r"I cannot find the end of this list:"),
                alloc.region_with_subregion(lines.convert_region(surroundings), lines.convert_region(region)),
                alloc.concat(vec![
                    alloc.reflow(r"You could change it to something like "),
                    alloc.parser_suggestion("[ 1, 2, 3 ]"),
                    alloc.reflow(" or even just "),
                    alloc.parser_suggestion("[]"),
                    alloc.reflow(". Anything where there is an open and a close square bracket, "),
                    alloc.reflow("and where the elements of the list are separated by commas."),
                ]),
                note_for_tag_union_type_indent(alloc),
            ]);

            Report {
                filename,
                doc,
                title: "UNFINISHED LIST".to_string(),
                severity: Severity::RuntimeError,
            }
        }
    }
}

fn to_if_report<'a>(
    alloc: &'a RocDocAllocator<'a>,
    lines: &LineInfo,
    filename: PathBuf,
    context: Context,
    parse_problem: &roc_parse::parser::EIf<'a>,
    start: Position,
) -> Report<'a> {
    use roc_parse::parser::EIf;

    match *parse_problem {
        EIf::Space(error, pos) => to_space_report(alloc, lines, filename, &error, pos),

        EIf::Condition(expr, pos) => to_expr_report(
            alloc,
            lines,
            filename,
            Context::InNode(Node::IfCondition, start, Box::new(context)),
            expr,
            pos,
        ),

        EIf::ThenBranch(expr, pos) => to_expr_report(
            alloc,
            lines,
            filename,
            Context::InNode(Node::IfThenBranch, start, Box::new(context)),
            expr,
            pos,
        ),

        EIf::ElseBranch(expr, pos) => to_expr_report(
            alloc,
            lines,
            filename,
            Context::InNode(Node::IfElseBranch, start, Box::new(context)),
            expr,
            pos,
        ),

        EIf::If(_pos) => unreachable!("another branch would be taken"),
        EIf::IndentIf(_pos) => unreachable!("another branch would be taken"),

        EIf::Then(pos) | EIf::IndentThenBranch(pos) | EIf::IndentThenToken(pos) => {
            to_unfinished_if_report(
                alloc,
                lines,
                filename,
                pos,
                start,
                alloc.concat(vec![
                    alloc.reflow(r"I was expecting to see the "),
                    alloc.keyword("then"),
                    alloc.reflow(r" keyword next."),
                ]),
            )
        }

        EIf::Else(pos) | EIf::IndentElseBranch(pos) | EIf::IndentElseToken(pos) => {
            to_unfinished_if_report(
                alloc,
                lines,
                filename,
                pos,
                start,
                alloc.concat(vec![
                    alloc.reflow(r"I was expecting to see the "),
                    alloc.keyword("else"),
                    alloc.reflow(r" keyword next."),
                ]),
            )
        }

        EIf::IndentCondition(pos) => to_unfinished_if_report(
            alloc,
            lines,
            filename,
            pos,
            start,
            alloc.concat(vec![
                alloc.reflow(r"I was expecting to see a expression next")
            ]),
        ),
    }
}

fn to_unfinished_if_report<'a>(
    alloc: &'a RocDocAllocator<'a>,
    lines: &LineInfo,
    filename: PathBuf,
    pos: Position,
    start: Position,
    message: RocDocBuilder<'a>,
) -> Report<'a> {
    let surroundings = Region::new(start, pos);
    let region = Region::from_pos(pos);

    let doc = alloc.stack(vec![
        alloc.concat(vec![
            alloc.reflow(r"I was partway through parsing an "),
            alloc.keyword("if"),
            alloc.reflow(r" expression, but I got stuck here:"),
        ]),
        alloc.region_with_subregion(lines.convert_region(surroundings), lines.convert_region(region)),
        message,
    ]);

    Report {
        filename,
        doc,
        title: "UNFINISHED IF".to_string(),
        severity: Severity::RuntimeError,
    }
}

fn to_when_report<'a>(
    alloc: &'a RocDocAllocator<'a>,
    lines: &LineInfo,
    filename: PathBuf,
    context: Context,
    parse_problem: &roc_parse::parser::EWhen<'a>,
    start: Position,
) -> Report<'a> {
    use roc_parse::parser::EWhen;

    match *parse_problem {
        EWhen::IfGuard(nested, pos) => match what_is_next(alloc.src_lines, lines.convert_pos(pos)) {
            Next::Token("->") => {
                let surroundings = Region::new(start, pos);
                let region = Region::from_pos(pos);

                let doc = alloc.stack(vec![
                    alloc.reflow(
                        r"I just started parsing an if guard, but there is no guard condition:",
                    ),
                    alloc.region_with_subregion(lines.convert_region(surroundings), lines.convert_region(region)),
                    alloc.concat(vec![
                        alloc.reflow("Try adding an expression before the arrow!")
                    ]),
                ]);

                Report {
                    filename,
                    doc,
                    title: "IF GUARD NO CONDITION".to_string(),
                    severity: Severity::RuntimeError,
                }
            }
            _ => to_expr_report(
                alloc,
                lines,
                filename,
                Context::InNode(Node::WhenIfGuard, start, Box::new(context)),
                nested,
                pos,
            ),
        },
        EWhen::Arrow(pos) => {
            let surroundings = Region::new(start, pos);
            let region = Region::from_pos(pos);

            let doc = alloc.stack(vec![
                alloc.concat(vec![
                    alloc.reflow(r"I am partway through parsing a "),
                    alloc.keyword("when"),
                    alloc.reflow(r" expression, but got stuck here:"),
                ]),
                alloc.region_with_subregion(lines.convert_region(surroundings), lines.convert_region(region)),
                alloc.concat(vec![alloc.reflow("I was expecting to see an arrow next.")]),
                note_for_when_indent_error(alloc),
            ]);

            Report {
                filename,
                doc,
                title: "MISSING ARROW".to_string(),
                severity: Severity::RuntimeError,
            }
        }

        EWhen::Space(error, pos) => to_space_report(alloc, lines, filename, &error, pos),

        EWhen::Branch(expr, pos) => to_expr_report(
            alloc,
            lines,
            filename,
            Context::InNode(Node::WhenBranch, start, Box::new(context)),
            expr,
            pos,
        ),

        EWhen::Condition(expr, pos) => to_expr_report(
            alloc,
            lines,
            filename,
            Context::InNode(Node::WhenCondition, start, Box::new(context)),
            expr,
            pos,
        ),

        EWhen::Bar(pos) => to_unfinished_when_report(
            alloc,
            lines,
            filename,
            pos,
            start,
            alloc.concat(vec![
                alloc.reflow(r"I just saw a "),
                alloc.parser_suggestion(r"|"),
                alloc.reflow(r" so I was expecting to see a pattern next."),
            ]),
        ),

        EWhen::IfToken(_pos) => unreachable!("the if-token is optional"),
        EWhen::When(_pos) => unreachable!("another branch would be taken"),

        EWhen::Is(pos) | EWhen::IndentIs(pos) => to_unfinished_when_report(
            alloc,
            lines,
            filename,
            pos,
            start,
            alloc.concat(vec![
                alloc.reflow(r"I was expecting to see the "),
                alloc.keyword("is"),
                alloc.reflow(r" keyword next."),
            ]),
        ),

        EWhen::IndentCondition(pos) => to_unfinished_when_report(
            alloc,
            lines,
            filename,
            pos,
            start,
            alloc.concat(vec![
                alloc.reflow(r"I was expecting to see a expression next")
            ]),
        ),

        EWhen::IndentPattern(pos) => to_unfinished_when_report(
            alloc,
            lines,
            filename,
            pos,
            start,
            alloc.concat(vec![alloc.reflow(r"I was expecting to see a pattern next")]),
        ),

        EWhen::IndentArrow(pos) => to_unfinished_when_report(
            alloc,
            lines,
            filename,
            pos,
            start,
            alloc.concat(vec![
                alloc.reflow(r"I just saw a pattern, so I was expecting to see a "),
                alloc.parser_suggestion("->"),
                alloc.reflow(" next."),
            ]),
        ),

        EWhen::IndentIfGuard(pos) => to_unfinished_when_report(
            alloc,
            lines,
            filename,
            pos,
            start,
            alloc.concat(vec![
                alloc.reflow(r"I just saw the "),
                alloc.keyword("if"),
                alloc.reflow(" keyword, so I was expecting to see an expression next."),
            ]),
        ),

        EWhen::IndentBranch(pos) => to_unfinished_when_report(
            alloc,
            lines,
            filename,
            pos,
            start,
            alloc.concat(vec![
                alloc.reflow(r"I was expecting to see an expression next. "),
                alloc.reflow("What should I do when I run into this particular pattern?"),
            ]),
        ),

        EWhen::PatternAlignment(indent, pos) => to_unfinished_when_report(
            alloc,
            lines,
            filename,
            pos,
            start,
            alloc.concat(vec![
                alloc.reflow(r"I suspect this is a pattern that is not indented enough? (by "),
                alloc.text(indent.to_string()),
                alloc.reflow(" spaces)"),
            ]),
        ),
        EWhen::Pattern(ref pat, pos) => to_pattern_report(alloc, lines, filename, pat, pos),
    }
}

fn to_unfinished_when_report<'a>(
    alloc: &'a RocDocAllocator<'a>,
    lines: &LineInfo,
    filename: PathBuf,
    pos: Position,
    start: Position,
    message: RocDocBuilder<'a>,
) -> Report<'a> {
    match what_is_next(alloc.src_lines, lines.convert_pos(pos)) {
        Next::Token("->") => to_unexpected_arrow_report(alloc, lines, filename, pos, start),

        _ => {
            let surroundings = Region::new(start, pos);
            let region = Region::from_pos(pos);

            let doc = alloc.stack(vec![
                alloc.concat(vec![
                    alloc.reflow(r"I was partway through parsing a "),
                    alloc.keyword("when"),
                    alloc.reflow(r" expression, but I got stuck here:"),
                ]),
                alloc.region_with_subregion(lines.convert_region(surroundings), lines.convert_region(region)),
                message,
                note_for_when_error(alloc),
            ]);

            Report {
                filename,
                doc,
                title: "UNFINISHED WHEN".to_string(),
                severity: Severity::RuntimeError,
            }
        }
    }
}

fn to_unexpected_arrow_report<'a>(
    alloc: &'a RocDocAllocator<'a>,
    lines: &LineInfo,
    filename: PathBuf,
    pos: Position,
    start: Position,
) -> Report<'a> {
    let surroundings = Region::new(start, pos);
    let region = Region::new(pos, pos.bump_column(2));

    let doc = alloc.stack(vec![
        alloc.concat(vec![
            alloc.reflow(r"I am parsing a "),
            alloc.keyword("when"),
            alloc.reflow(r" expression right now, but this arrow is confusing me:"),
        ]),
        alloc.region_with_subregion(lines.convert_region(surroundings), lines.convert_region(region)),
        alloc.concat(vec![
            alloc.reflow(r"It makes sense to see arrows around here, "),
            alloc.reflow(r"so I suspect it is something earlier."),
            alloc.reflow(
                r"Maybe this pattern is indented a bit farther from the previous patterns?",
            ),
        ]),
        note_for_when_error(alloc),
    ]);

    Report {
        filename,
        doc,
        title: "UNEXPECTED ARROW".to_string(),
        severity: Severity::RuntimeError,
    }
}

fn note_for_when_error<'a>(alloc: &'a RocDocAllocator<'a>) -> RocDocBuilder<'a> {
    alloc.stack(vec![
        alloc.concat(vec![
            alloc.note("Here is an example of a valid "),
            alloc.keyword("when"),
            alloc.reflow(r" expression for reference."),
        ]),
        alloc.vcat(vec![
            alloc.text("when List.first plants is").indent(4),
            alloc.text("Ok n ->").indent(6),
            alloc.text("n").indent(8),
            alloc.text(""),
            alloc.text("Err _ ->").indent(6),
            alloc.text("200").indent(8),
        ]),
        alloc.concat(vec![
            alloc.reflow(
                "Notice the indentation. All patterns are aligned, and each branch is indented",
            ),
            alloc.reflow(" a bit more than the corresponding pattern. That is important!"),
        ]),
    ])
}

fn note_for_when_indent_error<'a>(alloc: &'a RocDocAllocator<'a>) -> RocDocBuilder<'a> {
    alloc.stack(vec![
        alloc.concat(vec![
            alloc.note("Sometimes I get confused by indentation, so try to make your "),
            alloc.keyword("when"),
            alloc.reflow(r" look something like this:"),
        ]),
        alloc.vcat(vec![
            alloc.text("when List.first plants is").indent(4),
            alloc.text("Ok n ->").indent(6),
            alloc.text("n").indent(8),
            alloc.text(""),
            alloc.text("Err _ ->").indent(6),
            alloc.text("200").indent(8),
        ]),
        alloc.concat(vec![
            alloc.reflow(
                "Notice the indentation. All patterns are aligned, and each branch is indented",
            ),
            alloc.reflow(" a bit more than the corresponding pattern. That is important!"),
        ]),
    ])
}

fn to_pattern_report<'a>(
    alloc: &'a RocDocAllocator<'a>,
    lines: &LineInfo,
    filename: PathBuf,
    parse_problem: &roc_parse::parser::EPattern<'a>,
    start: Position,
) -> Report<'a> {
    use roc_parse::parser::EPattern;

    match parse_problem {
        EPattern::Start(pos) => {
            let surroundings = Region::new(start, *pos);
            let region = Region::from_pos(*pos);

            let doc = alloc.stack(vec![
                alloc.reflow(r"I just started parsing a pattern, but I got stuck here:"),
                alloc.region_with_subregion(lines.convert_region(surroundings), lines.convert_region(region)),
                alloc.note("I may be confused by indentation"),
            ]);

            Report {
                filename,
                doc,
                title: "UNFINISHED PATTERN".to_string(),
                severity: Severity::RuntimeError,
            }
        }
        EPattern::Record(record, pos) => to_precord_report(alloc, lines, filename, record, *pos),
        EPattern::PInParens(inparens, pos) => {
            to_pattern_in_parens_report(alloc, lines, filename, inparens, *pos)
        }
        _ => todo!("unhandled parse error: {:?}", parse_problem),
    }
}

fn to_precord_report<'a>(
    alloc: &'a RocDocAllocator<'a>,
    lines: &LineInfo,
    filename: PathBuf,
    parse_problem: &roc_parse::parser::PRecord<'a>,
    start: Position,
) -> Report<'a> {
    use roc_parse::parser::PRecord;

    match *parse_problem {
        PRecord::Open(pos) => match what_is_next(alloc.src_lines, lines.convert_pos(pos)) {
            Next::Keyword(keyword) => {
                let surroundings = Region::new(start, pos);
                let region = to_keyword_region(lines.convert_pos(pos), keyword);

                let doc = alloc.stack(vec![
                    alloc.reflow(r"I just started parsing a record pattern, but I got stuck on this field name:"),
                    alloc.region_with_subregion(lines.convert_region(surroundings), region),
                    alloc.concat(vec![
                        alloc.reflow(r"Looks like you are trying to use "),
                        alloc.keyword(keyword),
                        alloc.reflow(" as a field name, but that is a reserved word. Try using a different name!"),
                    ]),
                ]);

                Report {
                    filename,
                    doc,
                    title: "UNFINISHED RECORD PATTERN".to_string(),
                    severity: Severity::RuntimeError,
                }
            }
            _ => {
                let surroundings = Region::new(start, pos);
                let region = Region::from_pos(pos);

                let doc = alloc.stack(vec![
                    alloc.reflow(r"I just started parsing a record pattern, but I got stuck here:"),
                    alloc.region_with_subregion(lines.convert_region(surroundings), lines.convert_region(region)),
                    record_patterns_look_like(alloc),
                ]);

                Report {
                    filename,
                    doc,
                    title: "UNFINISHED RECORD PATTERN".to_string(),
                    severity: Severity::RuntimeError,
                }
            }
        },

        PRecord::End(pos) => {
            let surroundings = Region::new(start, pos);
            let region = Region::from_pos(pos);

            match what_is_next(alloc.src_lines, lines.convert_pos(pos)) {
                Next::Other(Some(c)) if c.is_alphabetic() => {
                    let doc = alloc.stack(vec![
                        alloc.reflow(r"I am partway through parsing a record pattern, but I got stuck here:"),
                        alloc.region_with_subregion(lines.convert_region(surroundings), lines.convert_region(region)),
                        alloc.concat(vec![
                            alloc.reflow(
                                r"I was expecting to see a colon, question mark, comma or closing curly brace.",
                            ),
                        ]),
                    ]);

                    Report {
                        filename,
                        doc,
                        title: "UNFINISHED RECORD PATTERN".to_string(),
                        severity: Severity::RuntimeError,
                    }
                }
                _ => {
                    let doc = alloc.stack(vec![
                alloc.reflow("I am partway through parsing a record pattern, but I got stuck here:"),
                alloc.region_with_subregion(lines.convert_region(surroundings), lines.convert_region(region)),
                alloc.concat(vec![
                    alloc.reflow(
                        r"I was expecting to see a closing curly brace before this, so try adding a ",
                    ),
                    alloc.parser_suggestion("}"),
                    alloc.reflow(" and see if that helps?"),
                ]),
            ]);

                    Report {
                        filename,
                        doc,
                        title: "UNFINISHED RECORD PATTERN".to_string(),
                        severity: Severity::RuntimeError,
                    }
                }
            }
        }

        PRecord::Field(pos) => match what_is_next(alloc.src_lines, lines.convert_pos(pos)) {
            Next::Keyword(keyword) => {
                let surroundings = Region::new(start, pos);
                let region = to_keyword_region(lines.convert_pos(pos), keyword);

                let doc = alloc.stack(vec![
                    alloc.reflow(r"I just started parsing a record pattern, but I got stuck on this field name:"),
                    alloc.region_with_subregion(lines.convert_region(surroundings), region),
                    alloc.concat(vec![
                        alloc.reflow(r"Looks like you are trying to use "),
                        alloc.keyword(keyword),
                        alloc.reflow(" as a field name, but that is a reserved word. Try using a different name!"),
                    ]),
                ]);

                Report {
                    filename,
                    doc,
                    title: "UNFINISHED RECORD PATTERN".to_string(),
                    severity: Severity::RuntimeError,
                }
            }
            Next::Other(Some(',')) => todo!(),
            Next::Other(Some('}')) => unreachable!("or is it?"),
            _ => {
                let surroundings = Region::new(start, pos);
                let region = Region::from_pos(pos);

                let doc = alloc.stack(vec![
                    alloc.reflow(r"I am partway through parsing a record pattern, but I got stuck here:"),
                    alloc.region_with_subregion(lines.convert_region(surroundings), lines.convert_region(region)),
                    alloc.concat(vec![
                        alloc.reflow(r"I was expecting to see another record field defined next, so I am looking for a name like "),
                        alloc.parser_suggestion("userName"),
                        alloc.reflow(" or "),
                        alloc.parser_suggestion("plantHight"),
                        alloc.reflow("."),
                    ]),
                ]);

                Report {
                    filename,
                    doc,
                    title: "PROBLEM IN RECORD PATTERN".to_string(),
                    severity: Severity::RuntimeError,
                }
            }
        },

        PRecord::Colon(_) => {
            unreachable!("because `{ foo }` is a valid field; the colon is not required")
        }
        PRecord::Optional(_) => {
            unreachable!("because `{ foo }` is a valid field; the question mark is not required")
        }

        PRecord::Pattern(pattern, pos) => to_pattern_report(alloc, lines, filename, pattern, pos),

        PRecord::Expr(expr, pos) => to_expr_report(
            alloc,
            lines,
            filename,
            Context::InNode(
                Node::RecordConditionalDefault,
                start,
                Box::new(Context::InDef(pos)),
            ),
            expr,
            pos,
        ),

        PRecord::IndentOpen(pos) => {
            let surroundings = Region::new(start, pos);
            let region = Region::from_pos(pos);

            let doc = alloc.stack(vec![
                alloc.reflow(r"I just started parsing a record pattern, but I got stuck here:"),
                alloc.region_with_subregion(lines.convert_region(surroundings), lines.convert_region(region)),
                record_patterns_look_like(alloc),
                note_for_record_pattern_indent(alloc),
            ]);

            Report {
                filename,
                doc,
                title: "UNFINISHED RECORD PATTERN".to_string(),
                severity: Severity::RuntimeError,
            }
        }

        PRecord::IndentEnd(pos) => match next_line_starts_with_close_curly(alloc.src_lines, lines.convert_pos(pos)) {
            Some(curly_pos) => {
                let surroundings = LineColumnRegion::new(lines.convert_pos(start), curly_pos);
                let region = LineColumnRegion::from_pos(curly_pos);

                let doc = alloc.stack(vec![
                        alloc.reflow(
                            "I am partway through parsing a record pattern, but I got stuck here:",
                        ),
                        alloc.region_with_subregion(surroundings, region),
                        alloc.concat(vec![
                            alloc.reflow("I need this curly brace to be indented more. Try adding more spaces before it!"),
                        ]),
                    ]);

                Report {
                    filename,
                    doc,
                    title: "NEED MORE INDENTATION".to_string(),
                    severity: Severity::RuntimeError,
                }
            }
            None => {
                let surroundings = Region::new(start, pos);
                let region = Region::from_pos(pos);

                let doc = alloc.stack(vec![
                    alloc.reflow(
                        r"I am partway through parsing a record pattern, but I got stuck here:",
                    ),
                    alloc.region_with_subregion(lines.convert_region(surroundings), lines.convert_region(region)),
                    alloc.concat(vec![
                        alloc.reflow("I was expecting to see a closing curly "),
                        alloc.reflow("brace before this, so try adding a "),
                        alloc.parser_suggestion("}"),
                        alloc.reflow(" and see if that helps?"),
                    ]),
                    note_for_record_pattern_indent(alloc),
                ]);

                Report {
                    filename,
                    doc,
                    title: "UNFINISHED RECORD PATTERN".to_string(),
                    severity: Severity::RuntimeError,
                }
            }
        },

        PRecord::IndentColon(_) => {
            unreachable!("because `{ foo }` is a valid field; the colon is not required")
        }

        PRecord::IndentOptional(_) => {
            unreachable!("because `{ foo }` is a valid field; the question mark is not required")
        }

        PRecord::Space(error, pos) => to_space_report(alloc, lines, filename, &error, pos),
    }
}

fn to_pattern_in_parens_report<'a>(
    alloc: &'a RocDocAllocator<'a>,
    lines: &LineInfo,
    filename: PathBuf,
    parse_problem: &roc_parse::parser::PInParens<'a>,
    start: Position,
) -> Report<'a> {
    use roc_parse::parser::PInParens;

    match *parse_problem {
        PInParens::Open(pos) => {
            // `Open` case is for exhaustiveness, this case shouldn not be reachable practically.
            let surroundings = Region::new(start, pos);
            let region = Region::from_pos(pos);

            let doc = alloc.stack(vec![
                alloc.reflow(
                    r"I just started parsing a pattern in parentheses, but I got stuck here:",
                ),
                alloc.region_with_subregion(lines.convert_region(surroundings), lines.convert_region(region)),
                alloc.concat(vec![
                    alloc.reflow(r"A pattern in parentheses looks like "),
                    alloc.parser_suggestion("(Ok 32)"),
                    alloc.reflow(r" or "),
                    alloc.parser_suggestion("(\"hello\")"),
                    alloc.reflow(" so I was expecting to see an expression next."),
                ]),
            ]);

            Report {
                filename,
                doc,
                title: "UNFINISHED PARENTHESES".to_string(),
                severity: Severity::RuntimeError,
            }
        }

        PInParens::End(pos) => {
            let surroundings = Region::new(start, pos);
            let region = Region::from_pos(pos);

            let doc = alloc.stack(vec![
                alloc.reflow("I am partway through parsing a pattern in parentheses, but I got stuck here:"),
                alloc.region_with_subregion(lines.convert_region(surroundings), lines.convert_region(region)),
                alloc.concat(vec![
                    alloc.reflow(
                        r"I was expecting to see a closing parenthesis before this, so try adding a ",
                    ),
                    alloc.parser_suggestion(")"),
                    alloc.reflow(" and see if that helps?"),
                ]),
            ]);

            Report {
                filename,
                doc,
                title: "UNFINISHED PARENTHESES".to_string(),
                severity: Severity::RuntimeError,
            }
        }

        PInParens::Pattern(pattern, pos) => to_pattern_report(alloc, lines, filename, pattern, pos),

        PInParens::IndentOpen(pos) => {
            let surroundings = Region::new(start, pos);
            let region = Region::from_pos(pos);

            let doc = alloc.stack(vec![
                alloc.reflow(
                    r"I just started parsing a pattern in parentheses, but I got stuck here:",
                ),
                alloc.region_with_subregion(lines.convert_region(surroundings), lines.convert_region(region)),
                record_patterns_look_like(alloc),
                note_for_record_pattern_indent(alloc),
            ]);

            Report {
                filename,
                doc,
                title: "UNFINISHED PARENTHESES".to_string(),
                severity: Severity::RuntimeError,
            }
        }

        PInParens::IndentEnd(pos) => {
            match next_line_starts_with_close_parenthesis(alloc.src_lines, lines.convert_pos(pos)) {
                Some(curly_pos) => {
                    let surroundings = LineColumnRegion::new(lines.convert_pos(start), curly_pos);
                    let region = LineColumnRegion::from_pos(curly_pos);

                    let doc = alloc.stack(vec![
                        alloc.reflow(
                            "I am partway through parsing a pattern in parentheses, but I got stuck here:",
                        ),
                        alloc.region_with_subregion(surroundings, region),
                        alloc.concat(vec![
                            alloc.reflow("I need this parenthesis to be indented more. Try adding more spaces before it!"),
                        ]),
                    ]);

                    Report {
                        filename,
                        doc,
                        title: "NEED MORE INDENTATION".to_string(),
                        severity: Severity::RuntimeError,
                    }
                }
                None => {
                    let surroundings = Region::new(start, pos);
                    let region = Region::from_pos(pos);

                    let doc = alloc.stack(vec![
                        alloc.reflow(
                            r"I am partway through parsing a pattern in parentheses, but I got stuck here:",
                        ),
                        alloc.region_with_subregion(lines.convert_region(surroundings), lines.convert_region(region)),
                        alloc.concat(vec![
                            alloc.reflow("I was expecting to see a closing parenthesis "),
                            alloc.reflow("before this, so try adding a "),
                            alloc.parser_suggestion(")"),
                            alloc.reflow(" and see if that helps?"),
                        ]),
                        note_for_record_pattern_indent(alloc),
                    ]);

                    Report {
                        filename,
                        doc,
                        title: "UNFINISHED PARENTHESES".to_string(),
                        severity: Severity::RuntimeError,
                    }
                }
            }
        }

        PInParens::Space(error, pos) => to_space_report(alloc, lines, filename, &error, pos),
    }
}

fn to_type_report<'a>(
    alloc: &'a RocDocAllocator<'a>,
    lines: &LineInfo,
    filename: PathBuf,
    parse_problem: &roc_parse::parser::EType<'a>,
    start: Position,
) -> Report<'a> {
    use roc_parse::parser::EType;

    match parse_problem {
        EType::TRecord(record, pos) => to_trecord_report(alloc, lines, filename, record, *pos),
        EType::TTagUnion(tag_union, pos) => to_ttag_union_report(alloc, lines, filename, tag_union, *pos),
        EType::TInParens(tinparens, pos) => to_tinparens_report(alloc, lines, filename, tinparens, *pos),
        EType::TApply(tapply, pos) => to_tapply_report(alloc, lines, filename, tapply, *pos),
        EType::TInlineAlias(talias, _) => to_talias_report(alloc, lines, filename, talias),

        EType::TFunctionArgument(pos) => match what_is_next(alloc.src_lines, lines.convert_pos(*pos)) {
            Next::Other(Some(',')) => {
                let surroundings = Region::new(start, *pos);
                let region = Region::from_pos(*pos);

                let doc = alloc.stack(vec![
                    alloc.reflow(r"I just started parsing a function argument type, but I encountered two commas in a row:"),
                    alloc.region_with_subregion(lines.convert_region(surroundings), lines.convert_region(region)),
                    alloc.concat(vec![alloc.reflow("Try removing one of them.")]),
                ]);

                Report {
                    filename,
                    doc,
                    title: "DOUBLE COMMA".to_string(),
                    severity: Severity::RuntimeError,
                }
            }
            _ => todo!(),
        },

        EType::TStart(pos) => {
            let surroundings = Region::new(start, *pos);
            let region = Region::from_pos(*pos);

            let doc = alloc.stack(vec![
                alloc.reflow(r"I just started parsing a type, but I got stuck here:"),
                alloc.region_with_subregion(lines.convert_region(surroundings), lines.convert_region(region)),
                alloc.concat(vec![
                    alloc.reflow(r"I am expecting a type next, like "),
                    alloc.parser_suggestion("Bool"),
                    alloc.reflow(r" or "),
                    alloc.parser_suggestion("List a"),
                    alloc.reflow("."),
                ]),
            ]);

            Report {
                filename,
                doc,
                title: "UNFINISHED TYPE".to_string(),
                severity: Severity::RuntimeError,
            }
        }

        EType::TIndentStart(pos) => {
            let surroundings = Region::new(start, *pos);
            let region = Region::from_pos(*pos);

            let doc = alloc.stack(vec![
                alloc.reflow(r"I just started parsing a type, but I got stuck here:"),
                alloc.region_with_subregion(lines.convert_region(surroundings), lines.convert_region(region)),
                alloc.note("I may be confused by indentation"),
            ]);

            Report {
                filename,
                doc,
                title: "UNFINISHED TYPE".to_string(),
                severity: Severity::RuntimeError,
            }
        }

        EType::TIndentEnd(pos) => {
            let surroundings = Region::new(start, *pos);
            let region = Region::from_pos(*pos);

            let doc = alloc.stack(vec![
                alloc.reflow(r"I am partway through parsing a type, but I got stuck here:"),
                alloc.region_with_subregion(lines.convert_region(surroundings), lines.convert_region(region)),
                alloc.note("I may be confused by indentation"),
            ]);

            Report {
                filename,
                doc,
                title: "UNFINISHED TYPE".to_string(),
                severity: Severity::RuntimeError,
            }
        }

        EType::TAsIndentStart(pos) => {
            let surroundings = Region::new(start, *pos);
            let region = Region::from_pos(*pos);

            let doc = alloc.stack(vec![
                alloc.reflow(r"I just started parsing an inline type alias, but I got stuck here:"),
                alloc.region_with_subregion(lines.convert_region(surroundings), lines.convert_region(region)),
                alloc.note("I may be confused by indentation"),
            ]);

            Report {
                filename,
                doc,
                title: "UNFINISHED INLINE ALIAS".to_string(),
                severity: Severity::RuntimeError,
            }
        }

        EType::TBadTypeVariable(pos) => {
            let surroundings = Region::new(start, *pos);
            let region = Region::from_pos(*pos);

            let doc = alloc.stack(vec![
                alloc.reflow(r"I am expecting a type variable, but I got stuck here:"),
                alloc.region_with_subregion(lines.convert_region(surroundings), lines.convert_region(region)),
            ]);

            Report {
                filename,
                doc,
                title: "BAD TYPE VARIABLE".to_string(),
                severity: Severity::RuntimeError,
            }
        }

        _ => todo!("unhandled type parse error: {:?}", &parse_problem),
    }
}

fn to_trecord_report<'a>(
    alloc: &'a RocDocAllocator<'a>,
    lines: &LineInfo,
    filename: PathBuf,
    parse_problem: &roc_parse::parser::ETypeRecord<'a>,
    start: Position,
) -> Report<'a> {
    use roc_parse::parser::ETypeRecord;

    match *parse_problem {
        ETypeRecord::Open(pos) => match what_is_next(alloc.src_lines, lines.convert_pos(pos)) {
            Next::Keyword(keyword) => {
                let surroundings = Region::new(start, pos);
                let region = to_keyword_region(lines.convert_pos(pos), keyword);

                let doc = alloc.stack(vec![
                    alloc.reflow(r"I just started parsing a record type, but I got stuck on this field name:"),
                    alloc.region_with_subregion(lines.convert_region(surroundings), region),
                    alloc.concat(vec![
                        alloc.reflow(r"Looks like you are trying to use "),
                        alloc.keyword(keyword),
                        alloc.reflow(" as a field name, but that is a reserved word. Try using a different name!"),
                    ]),
                ]);

                Report {
                    filename,
                    doc,
                    title: "UNFINISHED RECORD TYPE".to_string(),
                    severity: Severity::RuntimeError,
                }
            }
            _ => {
                let surroundings = Region::new(start, pos);
                let region = Region::from_pos(pos);

                let doc = alloc.stack(vec![
                    alloc.reflow(r"I just started parsing a record type, but I got stuck here:"),
                    alloc.region_with_subregion(lines.convert_region(surroundings), lines.convert_region(region)),
                    alloc.concat(vec![
                        alloc.reflow(r"Record types look like "),
                        alloc.parser_suggestion("{ name : String, age : Int },"),
                        alloc.reflow(" so I was expecting to see a field name next."),
                    ]),
                ]);

                Report {
                    filename,
                    doc,
                    title: "UNFINISHED RECORD TYPE".to_string(),
                    severity: Severity::RuntimeError,
                }
            }
        },

        ETypeRecord::End(pos) => {
            let surroundings = Region::new(start, pos);
            let region = Region::from_pos(pos);

            match what_is_next(alloc.src_lines, lines.convert_pos(pos)) {
                Next::Other(Some(c)) if c.is_alphabetic() => {
                    let doc = alloc.stack(vec![
                        alloc.reflow(r"I am partway through parsing a record type, but I got stuck here:"),
                        alloc.region_with_subregion(lines.convert_region(surroundings), lines.convert_region(region)),
                        alloc.concat(vec![
                            alloc.reflow(
                                r"I was expecting to see a colon, question mark, comma or closing curly brace.",
                            ),
                        ]),
                    ]);

                    Report {
                        filename,
                        doc,
                        title: "UNFINISHED RECORD TYPE".to_string(),
                        severity: Severity::RuntimeError,
                    }
                }
                _ => {
                    let doc = alloc.stack(vec![
                alloc.reflow("I am partway through parsing a record type, but I got stuck here:"),
                alloc.region_with_subregion(lines.convert_region(surroundings), lines.convert_region(region)),
                alloc.concat(vec![
                    alloc.reflow(
                        r"I was expecting to see a closing curly brace before this, so try adding a ",
                    ),
                    alloc.parser_suggestion("}"),
                    alloc.reflow(" and see if that helps?"),
                ]),
            ]);

                    Report {
                        filename,
                        doc,
                        title: "UNFINISHED RECORD TYPE".to_string(),
                        severity: Severity::RuntimeError,
                    }
                }
            }
        }

        ETypeRecord::Field(pos) => match what_is_next(alloc.src_lines, lines.convert_pos(pos)) {
            Next::Keyword(keyword) => {
                let surroundings = Region::new(start, pos);
                let region = to_keyword_region(lines.convert_pos(pos), keyword);

                let doc = alloc.stack(vec![
                    alloc.reflow(r"I just started parsing a record type, but I got stuck on this field name:"),
                    alloc.region_with_subregion(lines.convert_region(surroundings), region),
                    alloc.concat(vec![
                        alloc.reflow(r"Looks like you are trying to use "),
                        alloc.keyword(keyword),
                        alloc.reflow(" as a field name, but that is a reserved word. Try using a different name!"),
                    ]),
                ]);

                Report {
                    filename,
                    doc,
                    title: "UNFINISHED RECORD TYPE".to_string(),
                    severity: Severity::RuntimeError,
                }
            }
            Next::Other(Some(',')) => todo!(),
            Next::Other(Some('}')) => unreachable!("or is it?"),
            _ => {
                let surroundings = Region::new(start, pos);
                let region = Region::from_pos(pos);

                let doc = alloc.stack(vec![
                    alloc.reflow(r"I am partway through parsing a record type, but I got stuck here:"),
                    alloc.region_with_subregion(lines.convert_region(surroundings), lines.convert_region(region)),
                    alloc.concat(vec![
                        alloc.reflow(r"I was expecting to see another record field defined next, so I am looking for a name like "),
                        alloc.parser_suggestion("userName"),
                        alloc.reflow(" or "),
                        alloc.parser_suggestion("plantHight"),
                        alloc.reflow("."),
                    ]),
                ]);

                Report {
                    filename,
                    doc,
                    title: "PROBLEM IN RECORD TYPE".to_string(),
                    severity: Severity::RuntimeError,
                }
            }
        },

        ETypeRecord::Colon(_) => {
            unreachable!("because `{ foo }` is a valid field; the colon is not required")
        }
        ETypeRecord::Optional(_) => {
            unreachable!("because `{ foo }` is a valid field; the question mark is not required")
        }

        ETypeRecord::Type(tipe, pos) => to_type_report(alloc, lines, filename, tipe, pos),

        ETypeRecord::IndentOpen(pos) => {
            let surroundings = Region::new(start, pos);
            let region = Region::from_pos(pos);

            let doc = alloc.stack(vec![
                alloc.reflow(r"I just started parsing a record type, but I got stuck here:"),
                alloc.region_with_subregion(lines.convert_region(surroundings), lines.convert_region(region)),
                alloc.concat(vec![
                    alloc.reflow(r"Record types look like "),
                    alloc.parser_suggestion("{ name : String, age : Int },"),
                    alloc.reflow(" so I was expecting to see a field name next."),
                ]),
                note_for_record_type_indent(alloc),
            ]);

            Report {
                filename,
                doc,
                title: "UNFINISHED RECORD TYPE".to_string(),
                severity: Severity::RuntimeError,
            }
        }

        ETypeRecord::IndentEnd(pos) => {
            match next_line_starts_with_close_curly(alloc.src_lines, lines.convert_pos(pos)) {
                Some(curly_pos) => {
                    let surroundings = LineColumnRegion::new(lines.convert_pos(start), curly_pos);
                    let region = LineColumnRegion::from_pos(curly_pos);

                    let doc = alloc.stack(vec![
                        alloc.reflow(
                            "I am partway through parsing a record type, but I got stuck here:",
                        ),
                        alloc.region_with_subregion(surroundings, region),
                        alloc.concat(vec![
                            alloc.reflow("I need this curly brace to be indented more. Try adding more spaces before it!"),
                        ]),
                    ]);

                    Report {
                        filename,
                        doc,
                        title: "NEED MORE INDENTATION".to_string(),
                        severity: Severity::RuntimeError,
                    }
                }
                None => {
                    let surroundings = Region::new(start, pos);
                    let region = Region::from_pos(pos);

                    let doc = alloc.stack(vec![
                        alloc.reflow(
                            r"I am partway through parsing a record type, but I got stuck here:",
                        ),
                        alloc.region_with_subregion(lines.convert_region(surroundings), lines.convert_region(region)),
                        alloc.concat(vec![
                            alloc.reflow("I was expecting to see a closing curly "),
                            alloc.reflow("brace before this, so try adding a "),
                            alloc.parser_suggestion("}"),
                            alloc.reflow(" and see if that helps?"),
                        ]),
                        note_for_record_type_indent(alloc),
                    ]);

                    Report {
                        filename,
                        doc,
                        title: "UNFINISHED RECORD TYPE".to_string(),
                        severity: Severity::RuntimeError,
                    }
                }
            }
        }

        ETypeRecord::IndentColon(_) => {
            unreachable!("because `{ foo }` is a valid field; the colon is not required")
        }

        ETypeRecord::IndentOptional(_) => {
            unreachable!("because `{ foo }` is a valid field; the question mark is not required")
        }

        ETypeRecord::Space(error, pos) => to_space_report(alloc, lines, filename, &error, pos),
    }
}

fn to_ttag_union_report<'a>(
    alloc: &'a RocDocAllocator<'a>,
    lines: &LineInfo,
    filename: PathBuf,
    parse_problem: &roc_parse::parser::ETypeTagUnion<'a>,
    start: Position,
) -> Report<'a> {
    use roc_parse::parser::ETypeTagUnion;

    match *parse_problem {
        ETypeTagUnion::Open(pos) => match what_is_next(alloc.src_lines, lines.convert_pos(pos)) {
            Next::Keyword(keyword) => {
                let surroundings = Region::new(start, pos);
                let region = to_keyword_region(lines.convert_pos(pos), keyword);

                let doc = alloc.stack(vec![
                    alloc.reflow(r"I just started parsing a tag union, but I got stuck on this field name:"),
                    alloc.region_with_subregion(lines.convert_region(surroundings), region),
                    alloc.concat(vec![
                        alloc.reflow(r"Looks like you are trying to use "),
                        alloc.keyword(keyword),
                        alloc.reflow(" as a tag name, but that is a reserved word. Tag names must start with a uppercase letter."),
                    ]),
                ]);

                Report {
                    filename,
                    doc,
                    title: "UNFINISHED TAG UNION TYPE".to_string(),
                    severity: Severity::RuntimeError,
                }
            }
            Next::Other(Some(c)) if c.is_alphabetic() => {
                debug_assert!(c.is_lowercase());

                let surroundings = Region::new(start, pos);
                let region = Region::from_pos(pos);

                let doc = alloc.stack(vec![
                    alloc.reflow(
                        r"I am partway through parsing a tag union type, but I got stuck here:",
                    ),
                    alloc.region_with_subregion(lines.convert_region(surroundings), lines.convert_region(region)),
                    alloc.reflow(r"I was expecting to see a tag name."),
                    hint_for_tag_name(alloc),
                ]);

                Report {
                    filename,
                    doc,
                    title: "WEIRD TAG NAME".to_string(),
                    severity: Severity::RuntimeError,
                }
            }
            _ => {
                let surroundings = Region::new(start, pos);
                let region = Region::from_pos(pos);

                let doc = alloc.stack(vec![
                    alloc.reflow(r"I just started parsing a tag union type, but I got stuck here:"),
                    alloc.region_with_subregion(lines.convert_region(surroundings), lines.convert_region(region)),
                    alloc.concat(vec![
                        alloc.reflow(r"Tag unions look like "),
                        alloc.parser_suggestion("[ Many I64, None ],"),
                        alloc.reflow(" so I was expecting to see a tag name next."),
                    ]),
                ]);

                Report {
                    filename,
                    doc,
                    title: "UNFINISHED TAG UNION TYPE".to_string(),
                    severity: Severity::RuntimeError,
                }
            }
        },

        ETypeTagUnion::End(pos) => {
            let surroundings = Region::new(start, pos);
            let region = Region::from_pos(pos);

            match what_is_next(alloc.src_lines, lines.convert_pos(pos)) {
                Next::Other(Some(c)) if c.is_alphabetic() => {
                    debug_assert!(c.is_lowercase());

                    let doc = alloc.stack(vec![
                        alloc.reflow(
                            r"I am partway through parsing a tag union type, but I got stuck here:",
                        ),
                        alloc.region_with_subregion(lines.convert_region(surroundings), lines.convert_region(region)),
                        alloc.reflow(r"I was expecting to see a tag name."),
                        hint_for_tag_name(alloc),
                    ]);

                    Report {
                        filename,
                        doc,
                        title: "WEIRD TAG NAME".to_string(),
                        severity: Severity::RuntimeError,
                    }
                }
                Next::Other(Some('@')) => {
                    let doc = alloc.stack(vec![
                        alloc.reflow(
                            r"I am partway through parsing a tag union type, but I got stuck here:",
                        ),
                        alloc.region_with_subregion(lines.convert_region(surroundings), lines.convert_region(region)),
                        alloc.reflow(r"I was expecting to see a private tag name."),
                        hint_for_private_tag_name(alloc),
                    ]);

                    Report {
                        filename,
                        doc,
                        title: "WEIRD TAG NAME".to_string(),
                        severity: Severity::RuntimeError,
                    }
                }
                _ => {
                    let doc = alloc.stack(vec![
                        alloc.reflow(r"I am partway through parsing a tag union type, but I got stuck here:"),
                        alloc.region_with_subregion(lines.convert_region(surroundings), lines.convert_region(region)),
                        alloc.concat(vec![
                                alloc.reflow(
                                    r"I was expecting to see a closing square bracket before this, so try adding a ",
                                ),
                                alloc.parser_suggestion("]"),
                                alloc.reflow(" and see if that helps?"),
                            ]),
                        ]);

                    Report {
                        filename,
                        doc,
                        title: "UNFINISHED TAG UNION TYPE".to_string(),
                        severity: Severity::RuntimeError,
                    }
                }
            }
        }

        ETypeTagUnion::Type(tipe, pos) => to_type_report(alloc, lines, filename, tipe, pos),

        ETypeTagUnion::IndentOpen(pos) => {
            let surroundings = Region::new(start, pos);
            let region = Region::from_pos(pos);

            let doc = alloc.stack(vec![
                alloc.reflow(r"I just started parsing a tag union type, but I got stuck here:"),
                alloc.region_with_subregion(lines.convert_region(surroundings), lines.convert_region(region)),
                alloc.concat(vec![
                    alloc.reflow(r"Tag unions look like "),
                    alloc.parser_suggestion("[ Many I64, None ],"),
                    alloc.reflow(" so I was expecting to see a tag name next."),
                ]),
                note_for_tag_union_type_indent(alloc),
            ]);

            Report {
                filename,
                doc,
                title: "UNFINISHED TAG UNION TYPE".to_string(),
                severity: Severity::RuntimeError,
            }
        }

        ETypeTagUnion::IndentEnd(pos) => {
            match next_line_starts_with_close_square_bracket(alloc.src_lines, lines.convert_pos(pos)) {
                Some(curly_pos) => {
                    let surroundings = LineColumnRegion::new(lines.convert_pos(start), curly_pos);
                    let region = LineColumnRegion::from_pos(curly_pos);

                    let doc = alloc.stack(vec![
                        alloc.reflow(
                            "I am partway through parsing a tag union type, but I got stuck here:",
                        ),
                        alloc.region_with_subregion(surroundings, region),
                        alloc.concat(vec![
                            alloc.reflow("I need this square bracket to be indented more. Try adding more spaces before it!"),
                        ]),
                    ]);

                    Report {
                        filename,
                        doc,
                        title: "NEED MORE INDENTATION".to_string(),
                        severity: Severity::RuntimeError,
                    }
                }
                None => {
                    let surroundings = Region::new(start, pos);
                    let region = Region::from_pos(pos);

                    let doc = alloc.stack(vec![
                        alloc.reflow(
                            r"I am partway through parsing a tag union type, but I got stuck here:",
                        ),
                        alloc.region_with_subregion(lines.convert_region(surroundings), lines.convert_region(region)),
                        alloc.concat(vec![
                            alloc.reflow("I was expecting to see a closing square "),
                            alloc.reflow("bracket before this, so try adding a "),
                            alloc.parser_suggestion("]"),
                            alloc.reflow(" and see if that helps?"),
                        ]),
                        note_for_tag_union_type_indent(alloc),
                    ]);

                    Report {
                        filename,
                        doc,
                        title: "UNFINISHED TAG UNION TYPE".to_string(),
                        severity: Severity::RuntimeError,
                    }
                }
            }
        }

        ETypeTagUnion::Space(error, pos) => to_space_report(alloc, lines, filename, &error, pos),
    }
}

fn to_tinparens_report<'a>(
    alloc: &'a RocDocAllocator<'a>,
    lines: &LineInfo,
    filename: PathBuf,
    parse_problem: &roc_parse::parser::ETypeInParens<'a>,
    start: Position,
) -> Report<'a> {
    use roc_parse::parser::ETypeInParens;

    match *parse_problem {
        ETypeInParens::Open(pos) => {
            match what_is_next(alloc.src_lines, lines.convert_pos(pos)) {
                Next::Keyword(keyword) => {
                    let surroundings = Region::new(start, pos);
                    let region = to_keyword_region(lines.convert_pos(pos), keyword);

                    let doc = alloc.stack(vec![
                    alloc.reflow(r"I just saw an open parenthesis, so I was expecting to see a type next."),
                    alloc.region_with_subregion(lines.convert_region(surroundings), region),
                    alloc.concat(vec![
                        alloc.reflow(r"Something like "),
                        alloc.parser_suggestion("(List Person)"),
                        alloc.text(" or "),
                        alloc.parser_suggestion("(Result I64 Str)"),
                    ]),
                ]);

                    Report {
                        filename,
                        doc,
                        title: "UNFINISHED PARENTHESES".to_string(),
                        severity: Severity::RuntimeError,
                    }
                }
                Next::Other(Some(c)) if c.is_alphabetic() => {
                    debug_assert!(c.is_lowercase());

                    let surroundings = Region::new(start, pos);
                    let region = Region::from_pos(pos);

                    let doc = alloc.stack(vec![
                    alloc.reflow(
                        r"I am partway through parsing a type in parentheses, but I got stuck here:",
                    ),
                    alloc.region_with_subregion(lines.convert_region(surroundings), lines.convert_region(region)),
                    alloc.reflow(r"I was expecting to see a tag name."),
                    hint_for_tag_name(alloc),
                ]);

                    Report {
                        filename,
                        doc,
                        title: "WEIRD TAG NAME".to_string(),
                        severity: Severity::RuntimeError,
                    }
                }
                _ => {
                    let surroundings = Region::new(start, pos);
                    let region = Region::from_pos(pos);

                    let doc = alloc.stack(vec![
                        alloc.reflow(
                            r"I just started parsing a type in parentheses, but I got stuck here:",
                        ),
                        alloc.region_with_subregion(lines.convert_region(surroundings), lines.convert_region(region)),
                        alloc.concat(vec![
                            alloc.reflow(r"Tag unions look like "),
                            alloc.parser_suggestion("[ Many I64, None ],"),
                            alloc.reflow(" so I was expecting to see a tag name next."),
                        ]),
                    ]);

                    Report {
                        filename,
                        doc,
                        title: "UNFINISHED PARENTHESES".to_string(),
                        severity: Severity::RuntimeError,
                    }
                }
            }
        }

        ETypeInParens::End(pos) => {
            let surroundings = Region::new(start, pos);
            let region = Region::from_pos(pos);

            match what_is_next(alloc.src_lines, lines.convert_pos(pos)) {
                Next::Other(Some(c)) if c.is_alphabetic() => {
                    debug_assert!(c.is_lowercase());

                    // TODO hint for tuples?
                    let doc = alloc.stack(vec![
                        alloc.reflow(
                            r"I am partway through parsing a type in parentheses, but I got stuck here:",
                        ),
                        alloc.region_with_subregion(lines.convert_region(surroundings), lines.convert_region(region)),
                        alloc.reflow(r"I was expecting to see a tag name."),
                        hint_for_tag_name(alloc),
                    ]);

                    Report {
                        filename,
                        doc,
                        title: "WEIRD TAG NAME".to_string(),
                        severity: Severity::RuntimeError,
                    }
                }
                _ => {
                    let doc = alloc.stack(vec![
                        alloc.reflow(r"I am partway through parsing a type in parentheses, but I got stuck here:"),
                        alloc.region_with_subregion(lines.convert_region(surroundings), lines.convert_region(region)),
                        alloc.concat(vec![
                                alloc.reflow(
                                    r"I was expecting to see a closing parenthesis before this, so try adding a ",
                                ),
                                alloc.parser_suggestion(")"),
                                alloc.reflow(" and see if that helps?"),
                            ]),
                        ]);

                    Report {
                        filename,
                        doc,
                        title: "UNFINISHED PARENTHESES".to_string(),
                        severity: Severity::RuntimeError,
                    }
                }
            }
        }

        ETypeInParens::Type(tipe, pos) => to_type_report(alloc, lines, filename, tipe, pos),

        ETypeInParens::IndentOpen(pos) => {
            let surroundings = Region::new(start, pos);
            let region = Region::from_pos(pos);

            let doc = alloc.stack(vec![
                alloc
                    .reflow(r"I just started parsing a type in parentheses, but I got stuck here:"),
                alloc.region_with_subregion(lines.convert_region(surroundings), lines.convert_region(region)),
                alloc.concat(vec![
                    alloc.reflow(r"Tag unions look like "),
                    alloc.parser_suggestion("[ Many I64, None ],"),
                    alloc.reflow(" so I was expecting to see a tag name next."),
                ]),
                note_for_tag_union_type_indent(alloc),
            ]);

            Report {
                filename,
                doc,
                title: "UNFINISHED PARENTHESES".to_string(),
                severity: Severity::RuntimeError,
            }
        }

        ETypeInParens::IndentEnd(pos) => {
            match next_line_starts_with_close_parenthesis(alloc.src_lines, lines.convert_pos(pos)) {
                Some(curly_pos) => {
                    let surroundings = LineColumnRegion::new(lines.convert_pos(start), curly_pos);
                    let region = LineColumnRegion::from_pos(curly_pos);

                    let doc = alloc.stack(vec![
                        alloc.reflow(
                            "I am partway through parsing a type in parentheses, but I got stuck here:",
                        ),
                        alloc.region_with_subregion(surroundings, region),
                        alloc.concat(vec![
                            alloc.reflow("I need this parenthesis to be indented more. Try adding more spaces before it!"),
                        ]),
                    ]);

                    Report {
                        filename,
                        doc,
                        title: "NEED MORE INDENTATION".to_string(),
                        severity: Severity::RuntimeError,
                    }
                }
                None => {
                    let surroundings = Region::new(start, pos);
                    let region = Region::from_pos(pos);

                    let doc = alloc.stack(vec![
                        alloc.reflow(
                            r"I am partway through parsing a type in parentheses, but I got stuck here:",
                        ),
                        alloc.region_with_subregion(lines.convert_region(surroundings), lines.convert_region(region)),
                        alloc.concat(vec![
                            alloc.reflow("I was expecting to see a parenthesis "),
                            alloc.reflow("before this, so try adding a "),
                            alloc.parser_suggestion(")"),
                            alloc.reflow(" and see if that helps?"),
                        ]),
                        note_for_tag_union_type_indent(alloc),
                    ]);

                    Report {
                        filename,
                        doc,
                        title: "UNFINISHED PARENTHESES".to_string(),
                        severity: Severity::RuntimeError,
                    }
                }
            }
        }

        ETypeInParens::Space(error, pos) => to_space_report(alloc, lines, filename, &error, pos),
    }
}

fn to_tapply_report<'a>(
    alloc: &'a RocDocAllocator<'a>,
    lines: &LineInfo,
    filename: PathBuf,
    parse_problem: &roc_parse::parser::ETypeApply,
    _start: Position,
) -> Report<'a> {
    use roc_parse::parser::ETypeApply;

    match *parse_problem {
        ETypeApply::DoubleDot(pos) => {
            let region = Region::from_pos(pos);

            let doc = alloc.stack(vec![
                alloc.reflow(r"I encountered two dots in a row:"),
                alloc.region(lines.convert_region(region)),
                alloc.concat(vec![alloc.reflow("Try removing one of them.")]),
            ]);

            Report {
                filename,
                doc,
                title: "DOUBLE DOT".to_string(),
                severity: Severity::RuntimeError,
            }
        }
        ETypeApply::TrailingDot(pos) => {
            let region = Region::from_pos(pos);

            let doc = alloc.stack(vec![
                alloc.reflow(r"I encountered a dot with nothing after it:"),
                alloc.region(lines.convert_region(region)),
                alloc.concat(vec![
                    alloc.reflow("Dots are used to refer to a type in a qualified way, like "),
                    alloc.parser_suggestion("Num.I64"),
                    alloc.text(" or "),
                    alloc.parser_suggestion("List.List a"),
                    alloc.reflow(". Try adding a type name next."),
                ]),
            ]);

            Report {
                filename,
                doc,
                title: "TRAILING DOT".to_string(),
                severity: Severity::RuntimeError,
            }
        }
        ETypeApply::StartIsNumber(pos) => {
            let region = Region::from_pos(pos);

            let doc = alloc.stack(vec![
                alloc.reflow(r"I encountered a number at the start of a qualified name segment:"),
                alloc.region(lines.convert_region(region)),
                alloc.concat(vec![
                    alloc.reflow("All parts of a qualified type name must start with an uppercase letter, like "),
                    alloc.parser_suggestion("Num.I64"),
                    alloc.text(" or "),
                    alloc.parser_suggestion("List.List a"),
                    alloc.text("."),
                ]),
            ]);

            Report {
                filename,
                doc,
                title: "WEIRD QUALIFIED NAME".to_string(),
                severity: Severity::RuntimeError,
            }
        }
        ETypeApply::StartNotUppercase(pos) => {
            let region = Region::from_pos(pos);

            let doc = alloc.stack(vec![
                alloc.reflow(r"I encountered a lowercase letter at the start of a qualified name segment:"),
                alloc.region(lines.convert_region(region)),
                alloc.concat(vec![
                    alloc.reflow("All parts of a qualified type name must start with an uppercase letter, like "),
                    alloc.parser_suggestion("Num.I64"),
                    alloc.text(" or "),
                    alloc.parser_suggestion("List.List a"),
                    alloc.text("."),
                ]),
            ]);

            Report {
                filename,
                doc,
                title: "WEIRD QUALIFIED NAME".to_string(),
                severity: Severity::RuntimeError,
            }
        }

        ETypeApply::End(pos) => {
            let region = Region::from_pos(pos);

            let doc = alloc.stack(vec![
                alloc.reflow(
                    r"I reached the end of the input file while parsing a qualified type name",
                ),
                alloc.region(lines.convert_region(region)),
            ]);

            Report {
                filename,
                doc,
                title: "END OF FILE".to_string(),
                severity: Severity::RuntimeError,
            }
        }

        ETypeApply::Space(error, pos) => to_space_report(alloc, lines, filename, &error, pos),
    }
}

fn to_talias_report<'a>(
    alloc: &'a RocDocAllocator<'a>,
    lines: &LineInfo,
    filename: PathBuf,
    parse_problem: &roc_parse::parser::ETypeInlineAlias,
) -> Report<'a> {
    use roc_parse::parser::ETypeInlineAlias;

    match *parse_problem {
        ETypeInlineAlias::NotAnAlias(pos) => {
            let region = Region::from_pos(pos);

            let doc = alloc.stack(vec![
                alloc.concat(vec![
                    alloc.reflow("The inline type after this "),
                    alloc.keyword("as"),
                    alloc.reflow(" is not a type alias:"),
                ]),
                alloc.region(lines.convert_region(region)),
                alloc.concat(vec![
                    alloc.reflow("Inline alias types must start with an uppercase identifier and be followed by zero or more type arguments, like "),
                    alloc.type_str("Point"),
                    alloc.reflow(" or "),
                    alloc.type_str("List a"),
                    alloc.reflow("."),
                ]),
            ]);

            Report {
                filename,
                doc,
                title: "NOT AN INLINE ALIAS".to_string(),
                severity: Severity::RuntimeError,
            }
        }
        ETypeInlineAlias::Qualified(pos) => {
            let region = Region::from_pos(pos);

            let doc = alloc.stack(vec![
                alloc.reflow(r"This type alias has a qualified name:"),
                alloc.region(lines.convert_region(region)),
                alloc.reflow("An alias introduces a new name to the current scope, so it must be unqualified."),
            ]);

            Report {
                filename,
                doc,
                title: "QUALIFIED ALIAS NAME".to_string(),
                severity: Severity::RuntimeError,
            }
        }
        ETypeInlineAlias::ArgumentNotLowercase(pos) => {
            let region = Region::from_pos(pos);

            let doc = alloc.stack(vec![
                alloc.reflow(r"This alias type argument is not lowercase:"),
                alloc.region(lines.convert_region(region)),
                alloc.reflow("All type arguments must be lowercase."),
            ]);

            Report {
                filename,
                doc,
                title: "TYPE ARGUMENT NOT LOWERCASE".to_string(),
                severity: Severity::RuntimeError,
            }
        }
    }
}

fn to_header_report<'a>(
    alloc: &'a RocDocAllocator<'a>,
    lines: &LineInfo,
    filename: PathBuf,
    parse_problem: &roc_parse::parser::EHeader<'a>,
    start: Position,
) -> Report<'a> {
    use roc_parse::parser::EHeader;

    match parse_problem {
        EHeader::Provides(provides, pos) => to_provides_report(alloc, lines, filename, provides, *pos),

        EHeader::Exposes(exposes, pos) => to_exposes_report(alloc, lines, filename, exposes, *pos),

        EHeader::Imports(imports, pos) => to_imports_report(alloc, lines, filename, imports, *pos),

        EHeader::Requires(requires, pos) => to_requires_report(alloc, lines, filename, requires, *pos),

        EHeader::Packages(packages, pos) => to_packages_report(alloc, lines, filename, packages, *pos),

        EHeader::Effects(effects, pos) => to_effects_report(alloc, lines, filename, effects, *pos),

        EHeader::IndentStart(pos) => {
            let surroundings = Region::new(start, *pos);
            let region = Region::from_pos(*pos);

            let doc = alloc.stack(vec![
                alloc.reflow(r"I am partway through parsing a header, but got stuck here:"),
                alloc.region_with_subregion(lines.convert_region(surroundings), lines.convert_region(region)),
                alloc.concat(vec![alloc.reflow("I may be confused by indentation.")]),
            ]);

            Report {
                filename,
                doc,
                title: "INCOMPLETE HEADER".to_string(),
                severity: Severity::RuntimeError,
            }
        }

        EHeader::Start(pos) => {
            let surroundings = Region::new(start, *pos);
            let region = Region::from_pos(*pos);

            let doc = alloc.stack(vec![
                alloc.reflow(r"I am expecting a header, but got stuck here:"),
                alloc.region_with_subregion(lines.convert_region(surroundings), lines.convert_region(region)),
                alloc.concat(vec![
                    alloc.reflow("I am expecting a module keyword next, one of "),
                    alloc.keyword("interface"),
                    alloc.reflow(", "),
                    alloc.keyword("app"),
                    alloc.reflow(" or "),
                    alloc.keyword("platform"),
                    alloc.reflow("."),
                ]),
            ]);

            Report {
                filename,
                doc,
                title: "MISSING HEADER".to_string(),
                severity: Severity::RuntimeError,
            }
        }

        EHeader::ModuleName(pos) => {
            let surroundings = Region::new(start, *pos);
            let region = Region::from_pos(*pos);

            let doc = alloc.stack(vec![
                alloc.reflow(r"I am partway through parsing a header, but got stuck here:"),
                alloc.region_with_subregion(lines.convert_region(surroundings), lines.convert_region(region)),
                alloc.concat(vec![
                    alloc.reflow("I am expecting a module name next, like "),
                    alloc.parser_suggestion("BigNum"),
                    alloc.reflow(" or "),
                    alloc.parser_suggestion("Main"),
                    alloc.reflow(". Module names must start with an uppercase letter."),
                ]),
            ]);

            Report {
                filename,
                doc,
                title: "WEIRD MODULE NAME".to_string(),
                severity: Severity::RuntimeError,
            }
        }

        EHeader::AppName(_, pos) => {
            let surroundings = Region::new(start, *pos);
            let region = Region::from_pos(*pos);

            let doc = alloc.stack(vec![
                alloc.reflow(r"I am partway through parsing a header, but got stuck here:"),
                alloc.region_with_subregion(lines.convert_region(surroundings), lines.convert_region(region)),
                alloc.concat(vec![
                    alloc.reflow("I am expecting an application name next, like "),
                    alloc.parser_suggestion("app \"main\""),
                    alloc.reflow(" or "),
                    alloc.parser_suggestion("app \"editor\""),
                    alloc.reflow(". App names are surrounded by quotation marks."),
                ]),
            ]);

            Report {
                filename,
                doc,
                title: "WEIRD APP NAME".to_string(),
                severity: Severity::RuntimeError,
            }
        }

        EHeader::PlatformName(_, pos) => {
            let surroundings = Region::new(start, *pos);
            let region = Region::from_pos(*pos);

            let doc = alloc.stack(vec![
                alloc.reflow(r"I am partway through parsing a header, but got stuck here:"),
                alloc.region_with_subregion(lines.convert_region(surroundings), lines.convert_region(region)),
                alloc.concat(vec![
                    alloc.reflow("I am expecting a platform name next, like "),
                    alloc.parser_suggestion("\"roc/core\""),
                    alloc.reflow(". Platform names must be quoted."),
                ]),
            ]);

            Report {
                filename,
                doc,
                title: "WEIRD MODULE NAME".to_string(),
                severity: Severity::RuntimeError,
            }
        }

        EHeader::Space(error, pos) => to_space_report(alloc, lines, filename, error, *pos),
    }
}

fn to_provides_report<'a>(
    alloc: &'a RocDocAllocator<'a>,
    lines: &LineInfo,
    filename: PathBuf,
    parse_problem: &roc_parse::parser::EProvides,
    start: Position,
) -> Report<'a> {
    use roc_parse::parser::EProvides;

    match *parse_problem {
        EProvides::ListEnd(pos) | // TODO: give this its own error message
        EProvides::Identifier(pos) => {
            let surroundings = Region::new(start, pos);
            let region = Region::from_pos(pos);

            let doc = alloc.stack(vec![
                alloc
                    .reflow(r"I am partway through parsing a provides list, but I got stuck here:"),
                alloc.region_with_subregion(lines.convert_region(surroundings), lines.convert_region(region)),
                alloc.concat(vec![alloc.reflow(
                    "I was expecting a type name, value name or function name next, like",
                )]),
                alloc
                    .parser_suggestion("provides [ Animal, default, tame ]")
                    .indent(4),
            ]);

            Report {
                filename,
                doc,
                title: "WEIRD PROVIDES".to_string(),
                severity: Severity::RuntimeError,
            }
        }

        EProvides::Provides(pos) => {
            let surroundings = Region::new(start, pos);
            let region = Region::from_pos(pos);

            let doc = alloc.stack(vec![
                alloc.reflow(r"I am partway through parsing a header, but I got stuck here:"),
                alloc.region_with_subregion(lines.convert_region(surroundings), lines.convert_region(region)),
                alloc.concat(vec![
                    alloc.reflow("I am expecting the "),
                    alloc.keyword("provides"),
                    alloc.reflow(" keyword next, like "),
                ]),
                alloc
                    .parser_suggestion("provides [ Animal, default, tame ]")
                    .indent(4),
            ]);

            Report {
                filename,
                doc,
                title: "WEIRD PROVIDES".to_string(),
                severity: Severity::RuntimeError,
            }
        }

        EProvides::Space(error, pos) => to_space_report(alloc, lines, filename, &error, pos),

        _ => todo!("unhandled parse error {:?}", parse_problem),
    }
}

fn to_exposes_report<'a>(
    alloc: &'a RocDocAllocator<'a>,
    lines: &LineInfo,
    filename: PathBuf,
    parse_problem: &roc_parse::parser::EExposes,
    start: Position,
) -> Report<'a> {
    use roc_parse::parser::EExposes;

    match *parse_problem {
        EExposes::ListEnd(pos) | // TODO: give this its own error message
        EExposes::Identifier(pos) => {
            let surroundings = Region::new(start, pos);
            let region = Region::from_pos(pos);

            let doc = alloc.stack(vec![
                alloc.reflow(r"I am partway through parsing a exposes list, but I got stuck here:"),
                alloc.region_with_subregion(lines.convert_region(surroundings), lines.convert_region(region)),
                alloc.concat(vec![alloc.reflow(
                    "I was expecting a type name, value name or function name next, like",
                )]),
                alloc
                    .parser_suggestion("exposes [ Animal, default, tame ]")
                    .indent(4),
            ]);

            Report {
                filename,
                doc,
                title: "WEIRD EXPOSES".to_string(),
                severity: Severity::RuntimeError,
            }
        }

        EExposes::Exposes(pos) => {
            let surroundings = Region::new(start, pos);
            let region = Region::from_pos(pos);

            let doc = alloc.stack(vec![
                alloc.reflow(r"I am partway through parsing a header, but I got stuck here:"),
                alloc.region_with_subregion(lines.convert_region(surroundings), lines.convert_region(region)),
                alloc.concat(vec![
                    alloc.reflow("I am expecting the "),
                    alloc.keyword("exposes"),
                    alloc.reflow(" keyword next, like "),
                ]),
                alloc
                    .parser_suggestion("exposes [ Animal, default, tame ]")
                    .indent(4),
            ]);

            Report {
                filename,
                doc,
                title: "WEIRD EXPOSES".to_string(),
                severity: Severity::RuntimeError,
            }
        }

        EExposes::Space(error, pos) => to_space_report(alloc, lines, filename, &error, pos),

        _ => todo!("unhandled parse error {:?}", parse_problem),
    }
}

fn to_imports_report<'a>(
    alloc: &'a RocDocAllocator<'a>,
    lines: &LineInfo,
    filename: PathBuf,
    parse_problem: &roc_parse::parser::EImports,
    start: Position,
) -> Report<'a> {
    use roc_parse::parser::EImports;

    match *parse_problem {
        EImports::Identifier(pos) => {
            let surroundings = Region::new(start, pos);
            let region = Region::from_pos(pos);

            let doc = alloc.stack(vec![
                alloc.reflow(r"I am partway through parsing a imports list, but I got stuck here:"),
                alloc.region_with_subregion(lines.convert_region(surroundings), lines.convert_region(region)),
                alloc.concat(vec![alloc.reflow(
                    "I was expecting a type name, value name or function name next, like ",
                )]),
                alloc
                    .parser_suggestion("imports [ Animal, default, tame ]")
                    .indent(4),
            ]);

            Report {
                filename,
                doc,
                title: "WEIRD EXPOSES".to_string(),
                severity: Severity::RuntimeError,
            }
        }

        EImports::Imports(pos) => {
            let surroundings = Region::new(start, pos);
            let region = Region::from_pos(pos);

            let doc = alloc.stack(vec![
                alloc.reflow(r"I am partway through parsing a header, but I got stuck here:"),
                alloc.region_with_subregion(lines.convert_region(surroundings), lines.convert_region(region)),
                alloc.concat(vec![
                    alloc.reflow("I am expecting the "),
                    alloc.keyword("imports"),
                    alloc.reflow(" keyword next, like "),
                ]),
                alloc
                    .parser_suggestion("imports [ Animal, default, tame ]")
                    .indent(4),
            ]);

            Report {
                filename,
                doc,
                title: "WEIRD IMPORTS".to_string(),
                severity: Severity::RuntimeError,
            }
        }

        EImports::Space(error, pos) => to_space_report(alloc, lines, filename, &error, pos),

        EImports::ModuleName(pos) => {
            let surroundings = Region::new(start, pos);
            let region = Region::from_pos(pos);

            let doc = alloc.stack(vec![
                alloc.reflow(r"I am partway through parsing a header, but got stuck here:"),
                alloc.region_with_subregion(lines.convert_region(surroundings), lines.convert_region(region)),
                alloc.concat(vec![
                    alloc.reflow("I am expecting a module name next, like "),
                    alloc.parser_suggestion("BigNum"),
                    alloc.reflow(" or "),
                    alloc.parser_suggestion("Main"),
                    alloc.reflow(". Module names must start with an uppercase letter."),
                ]),
            ]);

            Report {
                filename,
                doc,
                title: "WEIRD MODULE NAME".to_string(),
                severity: Severity::RuntimeError,
            }
        }

        _ => todo!("unhandled parse error {:?}", parse_problem),
    }
}

fn to_requires_report<'a>(
    alloc: &'a RocDocAllocator<'a>,
    lines: &LineInfo,
    filename: PathBuf,
    parse_problem: &roc_parse::parser::ERequires<'a>,
    start: Position,
) -> Report<'a> {
    use roc_parse::parser::ERequires;

    match *parse_problem {
        ERequires::Requires(pos) => {
            let surroundings = Region::new(start, pos);
            let region = Region::from_pos(pos);

            let doc = alloc.stack(vec![
                alloc.reflow(r"I am partway through parsing a header, but I got stuck here:"),
                alloc.region_with_subregion(lines.convert_region(surroundings), lines.convert_region(region)),
                alloc.concat(vec![
                    alloc.reflow("I am expecting the "),
                    alloc.keyword("requires"),
                    alloc.reflow(" keyword next, like "),
                ]),
                alloc
                    .parser_suggestion("requires { main : Task I64 Str }")
                    .indent(4),
            ]);

            Report {
                filename,
                doc,
                title: "MISSING REQUIRES".to_string(),
                severity: Severity::RuntimeError,
            }
        }

        ERequires::Space(error, pos) => to_space_report(alloc, lines, filename, &error, pos),

        ERequires::ListStart(pos) => {
            let surroundings = Region::new(start, pos);
            let region = Region::from_pos(pos);

            let doc = alloc.stack(vec![
                alloc.reflow(r"I am partway through parsing a header, but I got stuck here:"),
                alloc.region_with_subregion(lines.convert_region(surroundings), lines.convert_region(region)),
                alloc.concat(vec![
                    alloc.reflow("I am expecting the "),
                    alloc.keyword("requires"),
                    alloc.reflow(" keyword next, like "),
                ]),
                alloc
                    .parser_suggestion("requires { main : Task I64 Str }")
                    .indent(4),
            ]);

            Report {
                filename,
                doc,
                title: "MISSING REQUIRES".to_string(),
                severity: Severity::RuntimeError,
            }
        }

        ERequires::Rigid(pos) => {
            let surroundings = Region::new(start, pos);
            let region = Region::from_pos(pos);

            let doc = alloc.stack(vec![
                alloc.reflow(r"I am partway through parsing a header, but I got stuck here:"),
                alloc.region_with_subregion(lines.convert_region(surroundings), lines.convert_region(region)),
                alloc.concat(vec![
                    alloc.reflow("I am expecting a list of rigids like "),
                    alloc.keyword("{}"),
                    alloc.reflow(" or "),
                    alloc.keyword("{model=>Model}"),
                    alloc.reflow(" next. A full "),
                    alloc.keyword("requires"),
                    alloc.reflow(" definition looks like"),
                ]),
                alloc
                    .parser_suggestion("requires {model=>Model, msg=>Msg} {main : Effect {}}")
                    .indent(4),
            ]);

            Report {
                filename,
                doc,
                title: "BAD REQUIRES RIGIDS".to_string(),
                severity: Severity::RuntimeError,
            }
        }

        _ => todo!("unhandled parse error {:?}", parse_problem),
    }
}

fn to_packages_report<'a>(
    alloc: &'a RocDocAllocator<'a>,
    lines: &LineInfo,
    filename: PathBuf,
    parse_problem: &roc_parse::parser::EPackages,
    start: Position,
) -> Report<'a> {
    use roc_parse::parser::EPackages;

    match *parse_problem {
        EPackages::Packages(pos) => {
            let surroundings = Region::new(start, pos);
            let region = Region::from_pos(pos);

            let doc = alloc.stack(vec![
                alloc.reflow(r"I am partway through parsing a header, but I got stuck here:"),
                alloc.region_with_subregion(lines.convert_region(surroundings), lines.convert_region(region)),
                alloc.concat(vec![
                    alloc.reflow("I am expecting the "),
                    alloc.keyword("packages"),
                    alloc.reflow(" keyword next, like "),
                ]),
                alloc.parser_suggestion("packages {}").indent(4),
            ]);

            Report {
                filename,
                doc,
                title: "MISSING PACKAGES".to_string(),
                severity: Severity::RuntimeError,
            }
        }

        EPackages::Space(error, pos) => to_space_report(alloc, lines, filename, &error, pos),

        _ => todo!("unhandled parse error {:?}", parse_problem),
    }
}

fn to_effects_report<'a>(
    alloc: &'a RocDocAllocator<'a>,
    lines: &LineInfo,
    filename: PathBuf,
    parse_problem: &roc_parse::parser::EEffects,
    start: Position,
) -> Report<'a> {
    use roc_parse::parser::EEffects;

    match *parse_problem {
        EEffects::Effects(pos) => {
            let surroundings = Region::new(start, pos);
            let region = Region::from_pos(pos);

            let doc = alloc.stack(vec![
                alloc.reflow(r"I am partway through parsing a header, but I got stuck here:"),
                alloc.region_with_subregion(lines.convert_region(surroundings), lines.convert_region(region)),
                alloc.concat(vec![
                    alloc.reflow("I am expecting the "),
                    alloc.keyword("effects"),
                    alloc.reflow(" keyword next, like "),
                ]),
                alloc.parser_suggestion("effects {}").indent(4),
            ]);

            Report {
                filename,
                doc,
                title: "MISSING PACKAGES".to_string(),
                severity: Severity::RuntimeError,
            }
        }

        EEffects::Space(error, pos) => to_space_report(alloc, lines, filename, &error, pos),

        _ => todo!("unhandled parse error {:?}", parse_problem),
    }
}

fn to_space_report<'a>(
    alloc: &'a RocDocAllocator<'a>,
    lines: &LineInfo,
    filename: PathBuf,
    parse_problem: &roc_parse::parser::BadInputError,
    pos: Position,
) -> Report<'a> {
    use roc_parse::parser::BadInputError;

    match parse_problem {
        BadInputError::HasTab => {
            let region = Region::from_pos(pos);

            let doc = alloc.stack(vec![
                alloc.reflow(r"I encountered a tab character"),
                alloc.region(lines.convert_region(region)),
                alloc.concat(vec![alloc.reflow("Tab characters are not allowed.")]),
            ]);

            Report {
                filename,
                doc,
                title: "TAB CHARACTER".to_string(),
                severity: Severity::RuntimeError,
            }
        }

        _ => todo!("unhandled type parse error: {:?}", &parse_problem),
    }
}

#[derive(Debug)]
enum Next<'a> {
    Keyword(&'a str),
    // Operator(&'a str),
    Close(&'a str, char),
    Token(&'a str),
    Other(Option<char>),
}

fn what_is_next<'a>(source_lines: &'a [&'a str], pos: LineColumn) -> Next<'a> {
    let row_index = pos.line as usize;
    let col_index = pos.column as usize;
    match source_lines.get(row_index) {
        None => Next::Other(None),
        Some(line) => {
            let chars = &line[col_index..];
            let mut it = chars.chars();

            match roc_parse::keyword::KEYWORDS
                .iter()
                .find(|keyword| starts_with_keyword(chars, keyword))
            {
                Some(keyword) => Next::Keyword(keyword),
                None => match it.next() {
                    None => Next::Other(None),
                    Some(c) => match c {
                        ')' => Next::Close("parenthesis", ')'),
                        ']' => Next::Close("square bracket", ']'),
                        '}' => Next::Close("curly brace", '}'),
                        '-' if it.next() == Some('>') => Next::Token("->"),
                        '=' if it.next() == Some('>') => Next::Token("=>"),
                        // _ if is_symbol(c) => todo!("it's an operator"),
                        _ => Next::Other(Some(c)),
                    },
                },
            }
        }
    }
}

pub fn starts_with_keyword(rest_of_line: &str, keyword: &str) -> bool {
    if let Some(stripped) = rest_of_line.strip_prefix(keyword) {
        match stripped.chars().next() {
            None => true,
            Some(c) => !c.is_alphanumeric(),
        }
    } else {
        false
    }
}

fn next_line_starts_with_close_curly(source_lines: &[&str], pos: LineColumn) -> Option<LineColumn> {
    next_line_starts_with_char(source_lines, pos, '}')
}

fn next_line_starts_with_close_parenthesis(
    source_lines: &[&str],
    pos: LineColumn,
) -> Option<LineColumn> {
    next_line_starts_with_char(source_lines, pos, ')')
}

fn next_line_starts_with_close_square_bracket(
    source_lines: &[&str],
    pos: LineColumn,
) -> Option<LineColumn> {
    next_line_starts_with_char(source_lines, pos, ']')
}

fn next_line_starts_with_char(
    source_lines: &[&str],
    pos: LineColumn,
    character: char,
) -> Option<LineColumn> {
    match source_lines.get(pos.line as usize + 1) {
        None => None,

        Some(line) => {
            let spaces_dropped = line.trim_start_matches(' ');
            match spaces_dropped.chars().next() {
                Some(c) if c == character => Some(LineColumn {
                    line: pos.line + 1,
                    column: (line.len() - spaces_dropped.len()) as u16,
                }),
                _ => None,
            }
        }
    }
}

fn to_keyword_region(pos: LineColumn, keyword: &str) -> LineColumnRegion {
    LineColumnRegion {
        start: pos,
        end: pos.bump_column(keyword.len() as u16)
    }
}
