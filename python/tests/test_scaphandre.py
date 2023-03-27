from scaphandre import RawScaphandre, Scaphandre


def test_scaphandre_should_init_with_the_good_name():
    assert Scaphandre().name == "PowercapRAPL"
