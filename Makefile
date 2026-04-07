test_disks_evaluation:
	@echo "Testing feature 'disks_evaluation'"
	cargo test -F disks_evaluation -- --test-threads=1
