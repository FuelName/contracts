library;

pub enum ResolveError {
    OwnerAndResolutionMismatch: (),
}

pub enum OwnershipError {
    NotDomainOwner: (),
}

pub enum ExpirationError {
    ExpiredDomain: (),
}
