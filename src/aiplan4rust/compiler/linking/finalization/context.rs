use crate::aiplan4rust::support::diagnostic::Provider;
use crate::aiplan4rust::support::interner::SymbolInterner;
use crate::aiplan4rust::support::lang::LiteralId;

pub struct FinalizationContext<'a> {
    interner: &'a SymbolInterner,
    source: LiteralId,
    provider: Provider,
}

impl<'a> FinalizationContext<'a> {
    pub fn new(interner: &'a SymbolInterner, source: LiteralId, provider: Provider) -> Self {
        Self {
            interner,
            source,
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
