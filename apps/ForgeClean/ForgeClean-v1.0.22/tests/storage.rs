use forgeclean::storage::{AutoMode, parse_lsblk_pairs, select_auto_mode};
use std::path::PathBuf;

#[test]
fn usb_partition_inherits_external_parent() {
    let sample = r#"NAME="sdb" PATH="/dev/sdb" PKNAME="" TYPE="disk" RM="0" HOTPLUG="1" TRAN="usb" MOUNTPOINT="" UUID=""
NAME="sdb1" PATH="/dev/sdb1" PKNAME="sdb" TYPE="part" RM="0" HOTPLUG="1" TRAN="" MOUNTPOINT="/run/media/benji/AetherVault" UUID="ABCD-1234""#;
    let rows = parse_lsblk_pairs(sample).unwrap();
    let mode = select_auto_mode(&rows, None);
    match mode {
        AutoMode::Offload(drive) => {
            assert_eq!(
                drive.mount_point,
                PathBuf::from("/run/media/benji/AetherVault")
            );
            assert_eq!(drive.uuid.as_deref(), Some("ABCD-1234"));
        }
        AutoMode::CleanOnly => panic!("USB drive was not detected"),
    }
}

#[test]
fn internal_root_disk_does_not_enable_offload() {
    let sample = r#"NAME="nvme0n1" PATH="/dev/nvme0n1" PKNAME="" TYPE="disk" RM="0" HOTPLUG="0" TRAN="nvme" MOUNTPOINT="" UUID=""
NAME="nvme0n1p2" PATH="/dev/nvme0n1p2" PKNAME="nvme0n1" TYPE="part" RM="0" HOTPLUG="0" TRAN="" MOUNTPOINT="/" UUID="ROOT""#;
    let rows = parse_lsblk_pairs(sample).unwrap();
    assert!(matches!(select_auto_mode(&rows, None), AutoMode::CleanOnly));
}

#[test]
fn empty_inventory_is_clean_only() {
    let rows = parse_lsblk_pairs("").unwrap();
    assert!(matches!(select_auto_mode(&rows, None), AutoMode::CleanOnly));
}
