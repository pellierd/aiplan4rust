use crate::aiplan4rust::diagnostic::Provider;
use crate::aiplan4rust::interner::SymbolInterner;
use crate::aiplan4rust::lang::LiteralId;
use crate::aiplan4rust::semantic::SemanticContext;

pub struct PassContext<'a> {
    interner: &'a SymbolInterner,
    source: LiteralId,
    provider: Provider,
}

impl<'a> PassContext<'a> {
    pub fn new(interner: &'a SymbolInterner, source: LiteralId, provider: Provider) -> Self {
        Self {
            interner,
            source,
            provider,
        }
    }

    pub fn from_semantic_context(context: &'a SemanticContext, provider: Provider) -> Self {
        Self {
            interner: context.interner(),
            source: context.source_id(),
            provider,
        }
    }

    // Getters
    pub fn interner(&self) -> &SymbolInterner {
        self.interner
    }
    pub fn source(&self) -> LiteralId {
        self.source
    }
    pub fn provider(&self) -> Provider {
        self.provider
    }
}
