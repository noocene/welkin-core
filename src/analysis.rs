use std::collections::HashMap;

use crate::term::{Context, Definitions, Index, NormalizationError, Term};

#[derive(Debug)]
pub enum CheckError {
    NormalizationError(NormalizationError),
    InferenceError(InferenceError),
    NonFunctionLambda { term: Term, ty: Term },
    TypeError { expected: Term, got: Term },
    ErasureMismatch { lambda: Term, ty: Term },
}

impl From<NormalizationError> for CheckError {
    fn from(e: NormalizationError) -> Self {
        CheckError::NormalizationError(e)
    }
}

impl From<InferenceError> for CheckError {
    fn from(e: InferenceError) -> Self {
        CheckError::InferenceError(e)
    }
}

#[derive(Debug)]
pub enum InferenceError {
    UnboundReference(String),
    NormalizationError(NormalizationError),
    CheckError(Box<CheckError>),
    NonFunctionApplication(Term),
    Impossible(Term),
}

impl From<NormalizationError> for InferenceError {
    fn from(e: NormalizationError) -> Self {
        InferenceError::NormalizationError(e)
    }
}

impl From<CheckError> for InferenceError {
    fn from(e: CheckError) -> Self {
        InferenceError::CheckError(Box::new(e))
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
    ) -> Result<(), CheckError> {
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
                        Err(CheckError::ErasureMismatch {
                            lambda: self.clone(),
                            ty: ty.clone(),
                        })?;
                    }
                    let ctx = ctx.with(binding.clone());
                    let self_annotation = Term::Annotation {
                        checked: true,
                        expression: Box::new(self.clone()),
                        ty: Box::new(ty.clone()),
                    };
                    let argument_annotation = Term::Annotation {
                        checked: true,
                        ty: argument_type,
                        expression: Box::new(Term::Variable(ctx.resolve(&binding).unwrap())),
                    };
                    return_type.substitute(Index::top().child(), &self_annotation);
                    return_type.substitute_top(&argument_annotation);
                    let mut body = body.clone();
                    body.substitute_top(&argument_annotation);
                    body.check_inner(&*return_type, definitions, &ctx)?;
                } else {
                    Err(CheckError::NonFunctionLambda {
                        term: self.clone(),
                        ty: ty.clone(),
                    })?
                }
            }
            Duplicate { .. } => todo!("handle duplicate in typecheck"),
            _ => {
                let mut inferred = self.infer_inner(definitions, ctx)?;
                inferred.lazy_normalize(definitions)?;
                if inferred != reduced {
                    Err(CheckError::TypeError {
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
    ) -> Result<Term, InferenceError> {
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
                    Err(InferenceError::UnboundReference(name.clone()))?
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

                let self_annotation = Term::Annotation {
                    checked: true,
                    expression: Box::new(Term::Variable(
                        ret_ctx.resolve(self_binding).ok_or_else(|| {
                            InferenceError::UnboundReference(self_binding.clone())
                        })?,
                    )),
                    ty: Box::new(self.clone()),
                };
                let argument_annotation = Term::Annotation {
                    checked: true,
                    expression: Box::new(Term::Variable(
                        ret_ctx.resolve(argument_binding).ok_or_else(|| {
                            InferenceError::UnboundReference(argument_binding.clone())
                        })?,
                    )),
                    ty: argument_type.clone(),
                };
                argument_type.check_inner(&Universe, definitions, ctx)?;
                let mut return_type = return_type.clone();
                return_type.substitute(Index::top().child(), &self_annotation);
                return_type.substitute_top(&argument_annotation);
                return_type.check_inner(&Universe, definitions, &ret_ctx)?;
                Universe
            }
            Apply { function, argument } => {
                let mut function_type = function.infer_inner(definitions, ctx)?;
                function_type.lazy_normalize(definitions)?;
                if let Function {
                    argument_type,
                    return_type,
                    ..
                } = &function_type
                {
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
                    Err(InferenceError::NonFunctionApplication(*function.clone()))?
                }
            }
            Variable { .. } => self.clone(),
            _ => todo!("the rest of inference for {:?}", self),
        })
    }

    pub fn check<U: TypedDefinitions>(&self, ty: &Term, definitions: &U) -> Result<(), CheckError> {
        let mut context = Default::default();
        self.check_inner(&ty, definitions, &mut context)
    }

    pub fn infer<U: TypedDefinitions>(&self, definitions: &U) -> Result<Term, InferenceError> {
        let mut context = Default::default();
        self.infer_inner(definitions, &mut context)
    }
}
