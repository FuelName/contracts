library;

pub enum ValidationError {
    InvalidDomainName: (),
    InvalidPeriod: (),
    WrongFeeAmount: (),
    WrongFeeAsset: (),
}

pub enum GracePeriodError {
    InvalidGracePeriodDuration: (),
}

pub enum DomainRenewalError {
    CanNotRenewRootDomain: (),
    ExpirationIsTooFar: (),
}
