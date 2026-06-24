// SPDX-License-Identifier: Apache-2.0
pragma solidity ^0.8.27;

/// @title IHermitDataService
/// @notice Interface for the Hermit Data Service on The Graph Protocol's Horizon framework.
///
/// The inverse of every analytics service: indexes wallets that have gone quiet and serves wake alerts when dormant wallets stir. Sophisticated dormancy detection — whales idle 18+ months, DAOs with unreachable quorum, LPs missing rebalance cycles. Absence-of-activity as a first-class, paid signal.
///
/// Provider lifecycle:
///   provision → register → startService (per tier) → [collect]* → stopService → deregister
interface IHermitDataService {
    // -------------------------------------------------------------------------
    // Types
    // -------------------------------------------------------------------------

    /// @notice Data tiers determine what a provider offers.
    enum DataTier {
        DORMANCY,     // 0 — query current dormancy status/score for an address
        WAKE_ALERTS,  // 1 — stir notifications when a dormant wallet moves
        COHORTS       // 2 — aggregate/historical dormancy analytics (DAOs, LP cohorts)
    }

    /// @notice Active or historical service registration for a provider.
    struct ServiceRegistration {
        DataTier tier;
        string   endpoint;
        bool     active;
    }

    // -------------------------------------------------------------------------
    // Events
    // -------------------------------------------------------------------------

    event ProviderRegistered(address indexed provider, string endpoint, string geoHash);
    event ProviderDeregistered(address indexed provider);
    event PaymentsDestinationSet(address indexed provider, address indexed destination);
    event ServiceStarted(address indexed provider, DataTier tier, string endpoint);
    event ServiceStopped(address indexed provider, DataTier tier);
    event MinThawingPeriodSet(uint64 period);
    event FeesBurned(address indexed provider, uint256 amount);
    event FeesWithdrawn(address indexed to, uint256 amount);

    // -------------------------------------------------------------------------
    // Errors
    // -------------------------------------------------------------------------

    error ProviderAlreadyRegistered(address provider);
    error ProviderNotRegistered(address provider);
    error ActiveServicesExist(address provider);
    error RegistrationNotFound(address provider, DataTier tier);
    error InvalidServiceProvider(address expected, address actual);
    error InvalidPaymentType();
    error ThawingPeriodTooShort(uint64 required, uint64 actual);

    // -------------------------------------------------------------------------
    // Provider operations
    // -------------------------------------------------------------------------

    /// @notice Update the address that receives collected GRT fees.
    function setPaymentsDestination(address destination) external;

    // -------------------------------------------------------------------------
    // Governance
    // -------------------------------------------------------------------------

    /// @notice Update the minimum thawing period (lower-bounded by MIN_THAWING_PERIOD).
    function setMinThawingPeriod(uint64 period) external;

    /// @notice Withdraw accumulated data-service revenue to `to`.
    function withdrawFees(address to, uint256 amount) external;

    // -------------------------------------------------------------------------
    // Views
    // -------------------------------------------------------------------------

    function isRegistered(address provider) external view returns (bool);

    function getServiceRegistrations(address provider)
        external
        view
        returns (ServiceRegistration[] memory);

    function activeServiceCount(address provider) external view returns (uint256);

    function paymentsDestination(address provider) external view returns (address);

    function minThawingPeriod() external view returns (uint64);
}
