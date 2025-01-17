// Copyright (C) Moondance Labs Ltd.
// This file is part of Tanssi.

// Tanssi is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

// Tanssi is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.

// You should have received a copy of the GNU General Public License
// along with Tanssi.  If not, see <http://www.gnu.org/licenses/>

use crate::common::xcm::*;

use {
    crate::common::xcm::mocknets::{Dancebox, Westend, WestendPallet},
    frame_support::{
        assert_ok,
        weights::{Weight, WeightToFee},
    },
    xcm::{
        latest::{prelude::*, Error::Trap as TrapError},
        VersionedMultiLocation, VersionedXcm,
    },
};

#[test]
fn trapping_asserts_works_with_polkadot_xcm() {
    // XcmPallet send arguments
    let sudo_origin = <Westend as Relay>::RuntimeOrigin::root();
    let dancebox_para_destination: VersionedMultiLocation =
        Westend::child_location_of(Dancebox::para_id()).into();

    let buy_execution_fee_amount =
        dancebox_runtime::WeightToFee::weight_to_fee(&Weight::from_parts(10_000_000_000, 300_000));

    let buy_execution_fee = MultiAsset {
        id: Concrete(dancebox_runtime::xcm_config::SelfReserve::get()),
        fun: Fungible(buy_execution_fee_amount),
    };

    let xcm = VersionedXcm::from(Xcm(vec![
        WithdrawAsset {
            0: vec![buy_execution_fee.clone()].into(),
        },
        BuyExecution {
            fees: buy_execution_fee.clone(),
            weight_limit: Unlimited,
        },
        Trap(0),
    ]));

    // Send XCM message from Relay Chain
    Westend::execute_with(|| {
        assert_ok!(<Westend as WestendPallet>::XcmPallet::send(
            sudo_origin,
            bx!(dancebox_para_destination),
            bx!(xcm),
        ));

        type RuntimeEvent = <Westend as Relay>::RuntimeEvent;

        assert_expected_events!(
            Westend,
            vec![
                RuntimeEvent::XcmPallet(pallet_xcm::Event::Sent { .. }) => {},
            ]
        );
    });

    // Receive XCM message in Assets Parachain
    Dancebox::execute_with(|| {
        type RuntimeEvent = <Dancebox as Para>::RuntimeEvent;
        assert_expected_events!(
            Dancebox,
            vec![
                RuntimeEvent::DmpQueue(
                    cumulus_pallet_dmp_queue::Event::ExecutedDownward {
                        outcome: Outcome::Incomplete(_w, error), ..
                    }) => {
                    error: *error == TrapError(0),
                },
                RuntimeEvent::PolkadotXcm(
                    pallet_xcm::Event::AssetsTrapped(_hash, origin, _assets)) => {
                        origin: *origin == MultiLocation::parent(),
                },
            ]
        );
    });
}
