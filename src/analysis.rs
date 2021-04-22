use derivative::Derivative;
use std::{collections::HashMap, fmt::Debug, hash::Hash};

use crate::term::{
    alloc::{Allocator, Reallocate, System, Zero},
    debug_reference, Index, None, NormalizationError, Primitives, Show, Term,
};

#[derive(Derivative)]
#[derivative(Debug(bound = "T: Show, U: Show"))]
pub enum AnalysisError<T, U: Primitives<T> = None, A: Allocator<T, U> = System> {
    NormalizationError(NormalizationError),
    NonFunctionLambda {
        term: Term<T, U, A>,
        ty: Term<T, U, A>,
    },
    TypeError {
        expected: Term<T, U, A>,
        got: Term<T, U, A>,
    },
    ErasureMismatch {
        lambda: Term<T, U, A>,
        ty: Term<T, U, A>,
    },
    UnboundReference(#[derivative(Debug(format_with = "debug_reference"))] T),
    NonFunctionApplication(Term<T, U, A>),
    UnboxedDuplication(Term<T, U, A>),
    Impossible(Term<T, U, A>),
    ExpectedWrap {
        term: Term<T, U, A>,
        ty: Term<T, U, A>,
    },
    InvalidWrap {
        wrap: Term<T, U, A>,
        got: Term<T, U, A>,
    },
}

impl<T, U: Primitives<T>, A: Allocator<T, U>> From<NormalizationError> for AnalysisError<T, U, A> {
    fn from(e: NormalizationError) -> Self {
        AnalysisError::NormalizationError(e)
    }
}

pub enum DefinitionResult<'a, T> {
    Borrowed(&'a T),
    Owned(T),
}

impl<'a, T> DefinitionResult<'a, T> {
    pub fn as_ref<'b>(&'b self) -> &'b T {
        match self {
            DefinitionResult::Borrowed(a) => a,
            DefinitionResult::Owned(a) => a,
        }
    }
}

pub trait Definitions<T, U: Primitives<T> = None, A: Allocator<T, U> = System> {
    fn get(&self, name: &T) -> Option<DefinitionResult<Term<T, U, A>>>;
}

pub struct Empty;

impl<T, U: Primitives<T>, A: Allocator<T, U>> TypedDefinitions<T, U, A> for Empty {
    fn get_typed(&self, _: &T) -> Option<DefinitionResult<(Term<T, U, A>, Term<T, U, A>)>> {
        None
    }
}

pub trait TypedDefinitions<T, U: Primitives<T> = None, A: Allocator<T, U> = System> {
    fn get_typed(&self, name: &T) -> Option<DefinitionResult<(Term<T, U, A>, Term<T, U, A>)>>;
}

impl<T: Hash + Eq, U: Primitives<T>, A: Allocator<T, U>> TypedDefinitions<T, U, A>
    for HashMap<T, (Term<T, U, A>, Term<T, U, A>)>
{
    fn get_typed(&self, name: &T) -> Option<DefinitionResult<(Term<T, U, A>, Term<T, U, A>)>> {
        HashMap::get(self, name).map(|a| DefinitionResult::Borrowed(a))
    }
}

impl<U, V: Primitives<U>, T: TypedDefinitions<U, V, A>, A: Allocator<U, V>> Definitions<U, V, A>
    for T
{
    fn get(&self, name: &U) -> Option<DefinitionResult<Term<U, V, A>>> {
        match TypedDefinitions::get_typed(self, name) {
            None => None,
            Some(DefinitionResult::Borrowed((_, a))) => Some(DefinitionResult::Borrowed(a)),
            Some(DefinitionResult::Owned((_, a))) => Some(DefinitionResult::Owned(a)),
        }
    }
}

impl<T, V: Primitives<T>, A: Allocator<T, V>> Term<T, V, A> {
    pub fn check_in<B: Allocator<T, V>, U: TypedDefinitions<T, V, B>>(
        &self,
        ty: &Term<T, V, A>,
        definitions: &U,
        alloc: &A,
    ) -> Result<(), AnalysisError<T, V, A>>
    where
        T: Show + Clone + PartialEq,
        V: Show + Clone,
        A: Reallocate<T, V, B>,
    {
        use Term::*;

        let mut reduced = alloc.copy(ty);
        reduced.weak_normalize_in(definitions, alloc)?;

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
                            lambda: alloc.copy(self),
                            ty: alloc.copy(ty),
                        })?;
                    }
                    let mut self_annotation = Term::Annotation {
                        checked: true,
                        expression: alloc.alloc(alloc.copy(self)),
                        ty: alloc.alloc(alloc.copy(ty)),
                    };
                    let argument_annotation = Term::Annotation {
                        checked: true,
                        ty: argument_type,
                        expression: alloc.alloc(Term::Variable(Index::top())),
                    };
                    self_annotation.shift_top();
                    return_type.substitute_in(Index::top().child(), &self_annotation, alloc);
                    return_type.substitute_top_in(&argument_annotation, alloc);
                    let mut body = alloc.copy(body);
                    body.substitute_top_in(&argument_annotation, alloc);
                    body.check_in(&*return_type, definitions, alloc)?;
                } else {
                    Err(AnalysisError::NonFunctionLambda {
                        term: alloc.copy(self),
                        ty: alloc.copy(ty),
                    })?
                }
            }
            Duplicate { expression, body } => {
                let expression_ty = expression.infer_in(definitions, alloc)?;
                let expression_ty = if let Wrap(term) = expression_ty {
                    term
                } else {
                    Err(AnalysisError::UnboxedDuplication(alloc.copy(self)))?
                };
                let argument_annotation = Term::Annotation {
                    checked: true,
                    ty: expression_ty,
                    expression: alloc.alloc(Term::Variable(Index::top())),
                };
                let mut body = alloc.copy(body);
                body.substitute_top_in(&argument_annotation, alloc);
                body.check_in(&reduced, definitions, alloc)?;
            }
            Put(term) => {
                if let Wrap(ty) = reduced {
                    term.check_in(&ty, definitions, alloc)?;
                } else {
                    Err(AnalysisError::ExpectedWrap {
                        term: alloc.copy(self),
                        ty: reduced,
                    })?;
                }
            }
            _ => {
                let inferred = self.infer_in(definitions, alloc)?;
                if !inferred.equivalent_in(&reduced, definitions, alloc)? {
                    Err(AnalysisError::TypeError {
                        expected: reduced,
                        got: inferred,
                    })?;
                }
            }
        })
    }

    pub fn infer_in<U: TypedDefinitions<T, V, B>, B: Allocator<T, V>>(
        &self,
        definitions: &U,
        alloc: &A,
    ) -> Result<Term<T, V, A>, AnalysisError<T, V, A>>
    where
        T: Show + Clone + PartialEq,
        V: Show + Clone,
        A: Reallocate<T, V, B>,
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
                    expression.check_in(ty, definitions, alloc)?;
                }
                alloc.copy(ty)
            }
            Reference(name) => {
                if let Some(dr) = definitions.get_typed(name) {
                    alloc.reallocating_copy(&dr.as_ref().0)
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
                    expression: alloc.alloc(Term::Variable(Index::top().child())),
                    ty: alloc.alloc(alloc.copy(self)),
                };
                let argument_annotation = Term::Annotation {
                    checked: true,
                    expression: alloc.alloc(Term::Variable(Index::top())),
                    ty: alloc.alloc(alloc.copy(argument_type)),
                };
                argument_type.check_in(&Universe, definitions, alloc)?;
                let mut return_type = alloc.copy(return_type);
                self_annotation.shift_top();
                return_type.substitute_in(Index::top().child(), &self_annotation, alloc);
                return_type.substitute_top_in(&argument_annotation, alloc);
                return_type.check_in(&Universe, definitions, alloc)?;
                Universe
            }
            Apply {
                function,
                argument,
                erased,
            } => {
                let mut function_type = function.infer_in(definitions, alloc)?;
                function_type.weak_normalize_in(definitions, alloc)?;
                if let Function {
                    argument_type,
                    return_type,
                    erased: function_erased,
                    ..
                } = &function_type
                {
                    if erased != function_erased {
                        Err(AnalysisError::ErasureMismatch {
                            lambda: alloc.copy(self),
                            ty: alloc.copy(&function_type),
                        })?;
                    }
                    let mut self_annotation = Term::Annotation {
                        expression: alloc.copy_boxed(function),
                        ty: alloc.alloc(alloc.copy(&function_type)),
                        checked: true,
                    };
                    let argument_annotation = Term::Annotation {
                        expression: alloc.copy_boxed(argument),
                        ty: alloc.copy_boxed(argument_type),
                        checked: true,
                    };
                    argument.check_in(argument_type, definitions, alloc)?;
                    let mut return_type = alloc.copy(return_type);
                    self_annotation.shift_top();
                    return_type.substitute_in(Index::top().child(), &self_annotation, alloc);
                    return_type.substitute_top_in(&argument_annotation, alloc);
                    return_type.weak_normalize_in(definitions, alloc)?;
                    return_type
                } else {
                    Err(AnalysisError::NonFunctionApplication(alloc.copy(function)))?
                }
            }
            Variable { .. } => alloc.copy(self),

            Wrap(expression) => {
                let expression_ty = expression.infer_in(definitions, alloc)?;
                if let Term::Universe = expression_ty {
                } else {
                    Err(AnalysisError::InvalidWrap {
                        got: expression_ty,
                        wrap: alloc.copy(self),
                    })?;
                }
                Universe
            }
            Put(expression) => Wrap(alloc.alloc(expression.infer_in(definitions, alloc)?)),

            Primitive(prim) => prim.ty(alloc),

            _ => Err(AnalysisError::Impossible(alloc.copy(self)))?,
        })
    }

    pub fn check<U: TypedDefinitions<T, V, A>>(
        &self,
        ty: &Term<T, V, A>,
        definitions: &U,
    ) -> Result<(), AnalysisError<T, V, A>>
    where
        T: Show + Clone + PartialEq + Debug,
        V: Show + Clone,
        A: Zero + Reallocate<T, V, A>,
    {
        let alloc = A::zero();

        self.check_in(ty, definitions, &alloc)
    }

    pub fn infer<U: TypedDefinitions<T, V, A>>(
        &self,
        definitions: &U,
    ) -> Result<Term<T, V, A>, AnalysisError<T, V, A>>
    where
        A: Zero + Reallocate<T, V, A>,
        T: Clone + PartialEq + Show + Debug,
        V: Clone + Show,
    {
        let alloc = A::zero();

        self.infer_in(definitions, &alloc)
    }
}
