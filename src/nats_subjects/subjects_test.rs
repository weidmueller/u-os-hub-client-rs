use super::*;
use rstest::rstest;

#[rstest]
#[case::provider_given_long_subject("v1.loc.provider1.vars.qry.read", Some("provider1".to_string()))]
#[case::provider_given_short_subject("v1.loc.provider1", Some("provider1".to_string()))]
#[case::no_provider_given_long_subject("v1.loc..def.evt.changed", None)]
#[case::provider_given_long_subject("v1.loc.registry.providers.provider1.vars.qry.read", Some("provider1".to_string()))]
#[case::provider_given_short_subject("v1.loc.registry.providers.provider1", Some("provider1".to_string()))]
#[case::provider_given_long_subject("v1.loc.registry.providers..vars.qry.read", None)]
#[case::no_provider_given_short_subject("v1.loc", None)]
fn test_get_provider_from_subject(
    #[case] subject: &str,
    #[case] expected_provider: Option<String>,
) {
    assert_eq!(get_provider_name_from_subject(subject), expected_provider);
}

#[rstest]
#[case::provider_given_long_subject("v1.loc.provi%der1.vars.qry.read", "provi%der1")]
#[case::provider_given_long_subject("v1.loc.test_pro-vider_1.vars.qry.read", "test_pro-vider_1".to_string())]
#[case::provider_given_long_subject("v1.loc.💩provider.vars.qry.read", "💩provider")]
#[case::provider_given_long_subject("v1.loc.registry.providers.registry.vars.qry.read", "registry".to_string())]
#[case::provider_given_short_subject("v1.loc.registry", "registry".to_string())]
#[case::provider_given_short_subject("v1.loc.abcdefghijklmnopqrstuvwxyzabcdefghijklmnopqrstuvwxyz0123456789", "abcdefghijklmnopqrstuvwxyzabcdefghijklmnopqrstuvwxyz0123456789".to_string())]
#[case::provider_given_short_subject(
    "v1.loc.abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ012345678912",
    "abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ012345678912"
)]
#[case::provider_given_short_subject("v1.loc._test1-test", "_test1-test")]
#[case::provider_given_short_subject("v1.loc.test1-test-", "test1-test-")]
#[case::provider_given_short_subject("v1.loc.0-test1-test-", "0-test1-test-")]
fn test_get_provider_id_from_subject(#[case] subject: &str, #[case] expected_provider: String) {
    let provider = get_provider_id_from_subject(subject).unwrap();
    assert_eq!(provider, expected_provider);
}
