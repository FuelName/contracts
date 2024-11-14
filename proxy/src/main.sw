contract;

use sway_libs::ownership::events::*;
use std::execution::run_external;
use standards::src5::{AccessError, State};
use standards::src14::{SRC14, SRC14_TARGET_STORAGE, SRC14Extension};

abi OwnershipOps {
    #[storage(read, write)]
    fn initialize_proxy();

    #[storage(read, write)]
    fn transfer_proxy_ownership(new_owner: Identity);

    #[storage(read, write)]
    fn renounce_proxy_ownership();
}

struct TargetChangedEvent {
    previous_target: ContractId,
    new_target: ContractId,
}

storage {
    SRC14 {
        /// The [ContractId] of the target contract.
        ///
        /// # Additional Information
        ///
        /// `target` is stored at sha256("storage_SRC14_0")
        target in SRC14_TARGET_STORAGE: ContractId = ContractId::zero(),
        /// The [State] of the proxy owner.
        owner: State = State::Uninitialized,
    },
}

impl SRC14 for Contract {
    #[storage(read, write)]
    fn set_proxy_target(new_target: ContractId) {
        only_owner();
        let previous_target = storage::SRC14.target.read();
        storage::SRC14.target.write(new_target);
        log(TargetChangedEvent {
            previous_target,
            new_target
        });
    }

    #[storage(read)]
    fn proxy_target() -> Option<ContractId> {
        storage::SRC14.target.try_read()
    }
}

impl SRC14Extension for Contract {
    #[storage(read)]
    fn proxy_owner() -> State {
        storage::SRC14.owner.read()
    }
}

impl OwnershipOps for Contract {
    #[storage(read, write)]
    fn initialize_proxy() {
        require(
            storage::SRC14
                .owner
                .read()
                .is_uninitialized(),
            AccessError::NotOwner,
        );
        let new_owner = msg_sender().unwrap();
        storage::SRC14
            .owner
            .write(State::Initialized(new_owner));
        log(OwnershipSet { new_owner });
    }

    #[storage(read, write)]
    fn transfer_proxy_ownership(new_owner: Identity) {
        only_owner();
        storage::SRC14.owner.write(State::Initialized(new_owner));
        log(OwnershipTransferred {
            new_owner,
            previous_owner: msg_sender().unwrap(),
        });
    }

    #[storage(read, write)]
    fn renounce_proxy_ownership() {
        only_owner();
        storage::SRC14.owner.write(State::Revoked);
        log(OwnershipRenounced {
            previous_owner: msg_sender().unwrap(),
        });
    }
}

#[fallback]
#[storage(read)]
fn fallback() {
    // pass through any other method call to the target
    run_external(storage::SRC14.target.read())
}

#[storage(read)]
fn only_owner() {
    require(
        storage::SRC14
            .owner
            .read() == State::Initialized(msg_sender().unwrap()),
        AccessError::NotOwner,
    );
}
