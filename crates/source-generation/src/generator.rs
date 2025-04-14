use crate::{SourceGenerationContext, SourceGenerationError};

pub trait ISourceGenerator {
    fn execute(
        &mut self,
        context: &mut SourceGenerationContext,
    ) -> Result<(), SourceGenerationError>;

    fn init(&mut self);

    fn name(&self) -> &'static str;
}

pub struct SourceGenerationDriver {
    generators: Vec<Box<dyn ISourceGenerator>>,
}

impl SourceGenerationDriver {
    pub fn new(generators: Vec<Box<dyn ISourceGenerator>>) -> Self {
        Self { generators }
    }

    pub fn execute(
        mut self,
        mut context: SourceGenerationContext,
        early_exit: bool,
    ) -> Result<(), Vec<(String, SourceGenerationError)>> {
        for generator in self.generators.iter_mut() {
            generator.init();
        }

        let mut errors = Vec::new();

        for generator in self.generators.iter_mut() {
            if let Err(err) = generator.execute(&mut context) {
                errors.push((generator.name().to_string(), err));

                if early_exit {
                    return Err(errors);
                }
            }
        }

        if let Err(err) = context.generate_mod_rs() {
            errors.push(("mod.rs".to_string(), err));
        }

        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }
}
