library;

pub enum ValidationError {
    InvalidDomainName: (),
    InvalidPeriod: (),
    WrongFeeAmount: (),
}

pub enum GracePeriodError {
    InvalidGracePeriodDuration: (),
}

pub enum DomainRenewalError {
    CanNotRenewRootDomain: (),
}
