library;

pub enum UnexpectedError {
    Unexpected: (),
}

pub enum MintError {
    AssetAlreadyMinted: (),
}

pub enum RenewalError {
    CanNotRenewRootDomain: (),
    InvalidExpirationValue: (),
    NoActiveDomainForRenewal: (),
    UnauthorizedTransactionSender: (),
}

pub enum ValidationError {
    InvalidDomainName: (),
    ExpirationNotSet: (),
    DomainNotPresent: (),
}

pub enum OwnershipError {
    NotDomainOwner: (),
}

pub enum AssetError {
    AssetDoesNotExist: (),
}

pub enum ResolutionError {
    AddressIsNotSet: (),
    CannotSetPrimaryForUnknownAddress: (),
    ExpiredDomain: (),
    ResolverIsNotSet: (),
}