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
