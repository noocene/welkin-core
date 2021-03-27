use derivative::Derivative;
use std::{collections::HashMap, hash::Hash};

use crate::term::{debug_reference, Index, NormalizationError, Show, Term};

#[derive(Derivative)]
#[derivative(Debug(bound = "T: Show"))]
pub enum AnalysisError<T> {
    NormalizationError(NormalizationError),
    NonFunctionLambda { term: Term<T>, ty: Term<T> },
    TypeError { expected: Term<T>, got: Term<T> },
    ErasureMismatch { lambda: Term<T>, ty: Term<T> },
    UnboundReference(#[derivative(Debug(format_with = "debug_reference"))] T),
    NonFunctionApplication(Term<T>),
    UnboxedDuplication(Term<T>),
    Impossible(Term<T>),
    ExpectedWrap { term: Term<T>, ty: Term<T> },
    InvalidWrap { wrap: Term<T>, got: Term<T> },
}

impl<T> From<NormalizationError> for AnalysisError<T> {
    fn from(e: NormalizationError) -> Self {
        AnalysisError::NormalizationError(e)
    }
}

pub trait Definitions<T> {
    fn get(&self, name: &T) -> Option<&Term<T>>;
}

pub trait TypedDefinitions<T> {
    fn get_typed(&self, name: &T) -> Option<&(Term<T>, Term<T>)>;
}

impl<T: Hash + Eq> TypedDefinitions<T> for HashMap<T, (Term<T>, Term<T>)> {
    fn get_typed(&self, name: &T) -> Option<&(Term<T>, Term<T>)> {
        HashMap::get(self, name)
    }
}

impl<U, T: TypedDefinitions<U>> Definitions<U> for T {
    fn get(&self, name: &U) -> Option<&Term<U>> {
        TypedDefinitions::get_typed(self, name).map(|(_, b)| b)
    }
}

impl<T> Term<T> {
    pub fn check<U: TypedDefinitions<T>>(
        &self,
        ty: &Term<T>,
        definitions: &U,
    ) -> Result<(), AnalysisError<T>>
    where
        T: Clone + Eq,
    {
        use Term::*;

        let mut reduced = ty.clone();
        reduced.lazy_normalize(definitions)?;

        Ok(match self {
            Lambda { body, erased } => {
                if let Function {
                    argument_type,
                    mut return_type,
                    erased: function_erased,
                } = reduced
                {
                    if *erased != function_erased {
                        Err(AnalysisError::ErasureMismatch {
                            lambda: self.clone(),
                            ty: ty.clone(),
                        })?;
                    }
                    let mut self_annotation = Term::Annotation {
                        checked: true,
                        expression: Box::new(self.clone()),
                        ty: Box::new(ty.clone()),
                    };
                    let argument_annotation = Term::Annotation {
                        checked: true,
                        ty: argument_type,
                        expression: Box::new(Term::Variable(Index::top())),
                    };
                    self_annotation.shift_top();
                    return_type.substitute(Index::top().child(), &self_annotation);
                    return_type.substitute_top(&argument_annotation);
                    let mut body = body.clone();
                    body.substitute_top(&argument_annotation);
                    body.check(&*return_type, definitions)?;
                } else {
                    Err(AnalysisError::NonFunctionLambda {
                        term: self.clone(),
                        ty: ty.clone(),
                    })?
                }
            }
            Duplicate { expression, body } => {
                let expression_ty = expression.infer(definitions)?;
                let expression_ty = if let Wrap(term) = expression_ty {
                    term
                } else {
                    Err(AnalysisError::UnboxedDuplication(self.clone()))?
                };
                let argument_annotation = Term::Annotation {
                    checked: true,
                    ty: expression_ty,
                    expression: Box::new(Term::Variable(Index::top())),
                };
                let mut body = body.clone();
                body.substitute_top(&argument_annotation);
                body.check(&reduced, definitions)?;
            }
            Put(term) => {
                if let Wrap(ty) = reduced {
                    term.check(&ty, definitions)?;
                } else {
                    Err(AnalysisError::ExpectedWrap {
                        term: self.clone(),
                        ty: reduced,
                    })?;
                }
            }
            _ => {
                let mut inferred = self.infer(definitions)?;
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

    pub fn infer<U: TypedDefinitions<T>>(
        &self,
        definitions: &U,
    ) -> Result<Term<T>, AnalysisError<T>>
    where
        T: Clone + Eq,
    {
        use Term::*;

        Ok(match self {
            Universe => Universe,
            Annotation {
                ty,
                expression,
                checked,
            } => {
                if !checked {
                    expression.check(ty, definitions)?;
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
                argument_type,
                return_type,
                ..
            } => {
                let mut self_annotation = Term::Annotation {
                    checked: true,
                    expression: Box::new(Term::Variable(Index::top().child())),
                    ty: Box::new(self.clone()),
                };
                let argument_annotation = Term::Annotation {
                    checked: true,
                    expression: Box::new(Term::Variable(Index::top())),
                    ty: argument_type.clone(),
                };
                argument_type.check(&Universe, definitions)?;
                let mut return_type = return_type.clone();
                self_annotation.shift_top();
                return_type.substitute(Index::top().child(), &self_annotation);
                return_type.substitute_top(&argument_annotation);
                return_type.check(&Universe, definitions)?;
                Universe
            }
            Apply {
                function,
                argument,
                erased,
            } => {
                let mut function_type = function.infer(definitions)?;
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
                    argument.check(argument_type, definitions)?;
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
                let mut expression_ty = expression.infer(definitions)?;
                expression_ty.lazy_normalize(definitions)?;
                if expression_ty != Universe {
                    Err(AnalysisError::InvalidWrap {
                        got: expression_ty,
                        wrap: self.clone(),
                    })?;
                }
                Universe
            }
            Put(expression) => Wrap(Box::new(expression.infer(definitions)?)),

            _ => Err(AnalysisError::Impossible(self.clone()))?,
        })
    }
}
