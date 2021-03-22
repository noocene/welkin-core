use std::collections::HashMap;

use super::{infer::InferenceError, normalize::NormalizationError, Term};

#[derive(Debug)]
pub enum CheckError {
    NormalizationError(NormalizationError),
    InferenceError(InferenceError),
    NonFunctionLambda,
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

impl Term {
    pub(crate) fn check_inner(
        &self,
        ty: &Term,
        definitions: &HashMap<String, (Term, Term)>,
        ctx: Vec<(String, Term)>,
    ) -> Result<(), CheckError> {
        use Term::*;

        Ok(match self {
            Lambda { binding, body } => {
                if let Function { .. } = ty {
                } else {
                    Err(CheckError::NonFunctionLambda)?
                }
            }
            _ => {
                let inferred = self.infer(definitions)?;
                println!("inferred: {:?}", inferred);
            }
        })
    }
    pub fn check(
        &self,
        ty: &Term,
        definitions: &HashMap<String, (Term, Term)>,
    ) -> Result<(), CheckError> {
        let ctx = vec![];
        self.check_inner(ty, definitions, ctx)
    }
}
