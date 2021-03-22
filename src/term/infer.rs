use std::collections::HashMap;

use super::{super::term, check::CheckError, normalize::NormalizationError, Term};

#[derive(Debug)]
pub enum InferenceError {
    UnboundReference(String),
    NormalizationError(NormalizationError),
    CheckError(Box<CheckError>),
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

impl Term {
    fn infer_inner(
        &self,
        definitions: &HashMap<String, (Term, Term)>,
        ctx: Vec<(String, Term)>,
    ) -> Result<Term, InferenceError> {
        use Term::*;

        Ok(match self {
            Universe => Universe,
            Wrap(term) => Wrap(Box::new(term.infer_inner(&definitions, ctx)?)),
            Function {
                self_binding,
                argument_binding,
                argument_type,
                return_type,
            } => {
                argument_type.check(&Universe, definitions)?;
                let reflexive_binding = Annotation {
                    checked: true,
                    expression: Box::new(Symbol(term::Symbol(ctx.len()))),
                    ty: Box::new(self.clone()),
                };
                let name_binding = Annotation {
                    checked: true,
                    expression: Box::new(Symbol(term::Symbol(ctx.len() + 1))),
                    ty: argument_type.clone(),
                };
                let mut ctx = ctx.clone();
                ctx.push((self_binding.clone(), self.clone()));
                ctx.push((argument_binding.clone(), *argument_type.clone()));
                let mut rt = return_type.clone();
                rt.substitute(&name_binding, 0);
                rt.substitute(&reflexive_binding, 0);
                rt.check_inner(&Universe, definitions, ctx)?;
                Universe
            }
            Symbol(a) => Symbol(*a),
            Reference(name) => {
                if let Some((ty, _)) = definitions.get(name) {
                    ty.clone()
                } else {
                    return Err(InferenceError::UnboundReference(name.clone()));
                }
            }
            Put(term) => Wrap(Box::new(term.infer_inner(&definitions, ctx)?)),
            _ => Err(InferenceError::Impossible(self.clone()))?,
        })
    }
    pub fn infer(
        &self,
        definitions: &HashMap<String, (Term, Term)>,
    ) -> Result<Term, InferenceError> {
        let ctx = vec![];
        self.infer_inner(definitions, ctx)
    }
}
