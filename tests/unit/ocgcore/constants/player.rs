use super::*;

#[test]
fn accepts_only_actual_players() {
    assert_eq!(Player::try_from(0).unwrap(), Player::Local);
    assert_eq!(Player::try_from(1).unwrap(), Player::Opponent);
    for value in 2..=u8::MAX {
        assert!(Player::try_from(value).is_err());
    }
}
