use std::collections::HashMap;

use crate::term::{Context, Definitions, Index, NormalizationError, Term};

#[derive(Debug)]
pub enum AnalysisError {
    NormalizationError(NormalizationError),
    NonFunctionLambda { term: Term, ty: Term },
    TypeError { expected: Term, got: Term },
    ErasureMismatch { lambda: Term, ty: Term },
    UnboundReference(String),
    NonFunctionApplication(Term),
    UnboxedDuplication(Term),
    Impossible(Term),
    ExpectedWrap { term: Term, ty: Term },
    InvalidWrap { wrap: Term, got: Term },
}

impl From<NormalizationError> for AnalysisError {
    fn from(e: NormalizationError) -> Self {
        AnalysisError::NormalizationError(e)
    }
}

pub(crate) mod sealed {
    use std::collections::HashMap;

    use crate::term::Term;

    pub trait SealedDefinitions {}

    impl SealedDefinitions for HashMap<String, (Term, Term)> {}
}

pub trait TypedDefinitions: sealed::SealedDefinitions {
    fn get_typed(&self, name: &str) -> Option<&(Term, Term)>;
}

impl TypedDefinitions for HashMap<String, (Term, Term)> {
    fn get_typed(&self, name: &str) -> Option<&(Term, Term)> {
        HashMap::get(self, name)
    }
}

impl<T: TypedDefinitions> Definitions for T {
    fn get(&self, name: &str) -> Option<&Term> {
        TypedDefinitions::get_typed(self, name).map(|(_, b)| b)
    }
}

impl Term {
    fn check_inner<U: TypedDefinitions>(
        &self,
        ty: &Term,
        definitions: &U,
        ctx: &Context,
    ) -> Result<(), AnalysisError> {
        use Term::*;

        let mut reduced = ty.clone();
        reduced.lazy_normalize(definitions)?;

        Ok(match self {
            Lambda {
                body,
                binding,
                erased,
            } => {
                if let Function {
                    argument_type,
                    mut return_type,
                    erased: function_erased,
                    ..
                } = reduced
                {
                    if *erased != function_erased {
                        Err(AnalysisError::ErasureMismatch {
                            lambda: self.clone(),
                            ty: ty.clone(),
                        })?;
                    }
                    let ctx = ctx.with(binding.clone());
                    let mut self_annotation = Term::Annotation {
                        checked: true,
                        expression: Box::new(self.clone()),
                        ty: Box::new(ty.clone()),
                    };
                    let argument_annotation = Term::Annotation {
                        checked: true,
                        ty: argument_type,
                        expression: Box::new(Term::Variable(ctx.resolve(&binding).unwrap())),
                    };
                    self_annotation.shift_top();
                    return_type.substitute(Index::top().child(), &self_annotation);
                    return_type.substitute_top(&argument_annotation);
                    let mut body = body.clone();
                    body.substitute_top(&argument_annotation);
                    body.check_inner(&*return_type, definitions, &ctx)?;
                } else {
                    Err(AnalysisError::NonFunctionLambda {
                        term: self.clone(),
                        ty: ty.clone(),
                    })?
                }
            }
            Duplicate {
                expression,
                binding,
                body,
            } => {
                let expression_ty = expression.infer_inner(definitions, ctx)?;
                let expression_ty = if let Wrap(term) = expression_ty {
                    term
                } else {
                    Err(AnalysisError::UnboxedDuplication(self.clone()))?
                };
                let ctx = ctx.with(binding.clone());
                let argument_annotation = Term::Annotation {
                    checked: true,
                    ty: expression_ty,
                    expression: Box::new(Term::Variable(ctx.resolve(&binding).unwrap())),
                };
                let mut body = body.clone();
                body.substitute_top(&argument_annotation);
                body.check_inner(&reduced, definitions, &ctx)?;
            }
            Put(term) => {
                if let Wrap(ty) = reduced {
                    term.check_inner(&ty, definitions, ctx)?;
                } else {
                    Err(AnalysisError::ExpectedWrap {
                        term: self.clone(),
                        ty: reduced,
                    })?;
                }
            }
            _ => {
                let mut inferred = self.infer_inner(definitions, ctx)?;
                inferred.lazy_normalize(definitions)?;
                if inferred != reduced {
                    Err(AnalysisError::TypeError {
                        expected: reduced.clone(),
                        got: inferred,
                    })?;
                }
            }
        })
    }

    fn infer_inner<U: TypedDefinitions>(
        &self,
        definitions: &U,
        ctx: &Context,
    ) -> Result<Term, AnalysisError> {
        use Term::*;

        Ok(match self {
            Universe => Universe,
            Annotation {
                ty,
                expression,
                checked,
            } => {
                if !checked {
                    expression.check_inner(ty, definitions, ctx)?;
                }
                *ty.clone()
            }
            Reference(name) => {
                if let Some((ty, _)) = definitions.get_typed(name) {
                    ty.clone()
                } else {
                    Err(AnalysisError::UnboundReference(name.clone()))?
                }
            }
            Function {
                self_binding,
                argument_binding,
                argument_type,
                return_type,
                ..
            } => {
                let mut ret_ctx = ctx.with(self_binding.clone());
                ret_ctx = ret_ctx.with(argument_binding.clone());

                let mut self_annotation = Term::Annotation {
                    checked: true,
                    expression: Box::new(Term::Variable(
                        ret_ctx
                            .resolve(self_binding)
                            .ok_or_else(|| AnalysisError::UnboundReference(self_binding.clone()))?,
                    )),
                    ty: Box::new(self.clone()),
                };
                let argument_annotation = Term::Annotation {
                    checked: true,
                    expression: Box::new(Term::Variable(
                        ret_ctx.resolve(argument_binding).ok_or_else(|| {
                            AnalysisError::UnboundReference(argument_binding.clone())
                        })?,
                    )),
                    ty: argument_type.clone(),
                };
                argument_type.check_inner(&Universe, definitions, ctx)?;
                let mut return_type = return_type.clone();
                self_annotation.shift_top();
                return_type.substitute(Index::top().child(), &self_annotation);
                return_type.substitute_top(&argument_annotation);
                return_type.check_inner(&Universe, definitions, &ret_ctx)?;
                Universe
            }
            Apply {
                function,
                argument,
                erased,
            } => {
                let mut function_type = function.infer_inner(definitions, ctx)?;
                function_type.lazy_normalize(definitions)?;
                if let Function {
                    argument_type,
                    return_type,
                    erased: function_erased,
                    ..
                } = &function_type
                {
                    if erased != function_erased {
                        Err(AnalysisError::ErasureMismatch {
                            lambda: self.clone(),
                            ty: function_type.clone(),
                        })?;
                    }
                    let mut self_annotation = Term::Annotation {
                        expression: function.clone(),
                        ty: Box::new(function_type.clone()),
                        checked: true,
                    };
                    let argument_annotation = Term::Annotation {
                        expression: argument.clone(),
                        ty: argument_type.clone(),
                        checked: true,
                    };
                    argument.check_inner(argument_type, definitions, ctx)?;
                    let mut return_type = return_type.clone();
                    self_annotation.shift_top();
                    return_type.substitute(Index::top().child(), &self_annotation);
                    return_type.substitute_top(&argument_annotation);
                    return_type.lazy_normalize(definitions)?;
                    *return_type
                } else {
                    Err(AnalysisError::NonFunctionApplication(*function.clone()))?
                }
            }
            Variable { .. } => self.clone(),

            Wrap(expression) => {
                let mut expression_ty = expression.infer_inner(definitions, ctx)?;
                expression_ty.lazy_normalize(definitions)?;
                if expression_ty != Universe {
                    Err(AnalysisError::InvalidWrap {
                        got: expression_ty,
                        wrap: self.clone(),
                    })?;
                }
                Universe
            }
            Put(expression) => Wrap(Box::new(expression.infer_inner(definitions, ctx)?)),

            _ => Err(AnalysisError::Impossible(self.clone()))?,
        })
    }

    pub fn check<U: TypedDefinitions>(
        &self,
        ty: &Term,
        definitions: &U,
    ) -> Result<(), AnalysisError> {
        let mut context = Default::default();
        self.check_inner(&ty, definitions, &mut context)
    }

    pub fn infer<U: TypedDefinitions>(&self, definitions: &U) -> Result<Term, AnalysisError> {
        let mut context = Default::default();
        self.infer_inner(definitions, &mut context)
    }
}
