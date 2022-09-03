//! # utils
//!
//! The utils module provides common functions used by the exporters.
use clap::crate_version;
#[cfg(feature = "containers")]
use {
    docker_sync::Docker,
    k8s_sync::{errors::KubernetesError, kubernetes::Kubernetes},
};

/// Returns an Option containing the VM name of a qemu process.
///
/// Then VM name is extracted from the command line.
pub fn filter_qemu_cmdline(cmdline: &str) -> Option<String> {
    if cmdline.contains("qemu-system") && cmdline.contains("guest=") {
        let vmname: Vec<Vec<&str>> = cmdline
            .split("guest=")
            .map(|x| x.split(',').collect())
            .collect();

        match (vmname[1].len(), vmname[1][0].is_empty()) {
            (1, _) => return None,
            (_, true) => return None,
            (_, false) => return Some(String::from(vmname[1][0])),
        }
    }
    None
}

/// Returns scaphandre version.
pub fn get_scaphandre_version() -> String {
    let mut version_parts = crate_version!().split('.');
    let major_version = version_parts.next().unwrap();
    let patch_version = version_parts.next().unwrap();
    let minor_version = version_parts.next().unwrap();
    format!("{}.{}{}", major_version, patch_version, minor_version)
}

/// Returns the hostname of the system running Scaphandre.
pub fn get_hostname() -> String {
    String::from(
        hostname::get()
            .expect("Fail to get system hostname")
            .to_str()
            .unwrap(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_filter_qemu_cmdline_ok() {
        let cmdline = "file=/var/lib/libvirt/qemu/domain-1-fedora33/master-key.aes-object-Sguest=fedora33,debug-threads=on-name/usr/bin/qemu-system-x86_64";
        assert_eq!(filter_qemu_cmdline(cmdline), Some("fedora33".to_string()));
    }

    #[test]
    fn test_filter_qemu_cmdline_ko_not_qemu() {
        let cmdline = "file=/var/lib/libvirt/qemu/domain-1-fedora33/master-key.aes-object-Sguest=fedora33,debug-threads=on-name/usr/bin/bidule";
        assert_eq!(filter_qemu_cmdline(cmdline), None);
    }

    #[test]
    fn test_filter_qemu_cmdline_ko_no_guest_token() {
        let cmdline = "file=/var/lib/libvirt/qemu/domain-1-fedora33/master-key.aes-object-Sfuest=fedora33,debug-threads=on-name/usr/bin/qemu-system-x86_64";
        assert_eq!(filter_qemu_cmdline(cmdline), None);
    }

    #[test]
    fn test_filter_qemu_cmdline_ko_no_comma_separator() {
        let cmdline = "file=/var/lib/libvirt/qemu/domain-1-fedora33/master-key.aes-object-Sguest=fedora33#debug-threads=on-name/usr/bin/qemu-system-x86_64";
        assert_eq!(filter_qemu_cmdline(cmdline), None);
    }

    #[test]
    fn test_filter_qemu_cmdline_ko_empty_guest01() {
        let cmdline = "file=/var/lib/libvirt/qemu/domain-1-fedora33/master-key.aes-object-Sguest=,,debug-threads=on-name/usr/bin/qemu-system-x86_64";
        assert_eq!(filter_qemu_cmdline(cmdline), None);
    }

    #[test]
    fn test_filter_qemu_cmdline_ko_empty_guest02() {
        let cmdline = "qemu-system-x86_64,file=/var/lib/libvirt/qemu/domain-1-fedora33/master-key.aes-object-Sguest=";
        assert_eq!(filter_qemu_cmdline(cmdline), None);
    }
}

#[cfg(feature = "containers")]
pub fn get_docker_client() -> Result<Docker, std::io::Error> {
    let docker = match Docker::connect() {
        Ok(docker) => docker,
        Err(err) => return Err(err),
    };
    Ok(docker)
}

#[cfg(feature = "containers")]
pub fn get_kubernetes_client() -> Result<Kubernetes, KubernetesError> {
    match Kubernetes::connect(
        Some(String::from("/root/.kube/config")),
        None,
        None,
        None,
        true,
    ) {
        Ok(kubernetes) => Ok(kubernetes),
        Err(err) => {
            eprintln!("Got Kubernetes error: {} | {:?}", err, err);
            Err(err)
        }
    }
}

//  Copyright 2020 The scaphandre authors.
//
//  Licensed under the Apache License, Version 2.0 (the "License");
//  you may not use this file except in compliance with the License.
//  You may obtain a copy of the License at
//
//      http://www.apache.org/licenses/LICENSE-2.0
//
//  Unless required by applicable law or agreed to in writing, software
//  distributed under the License is distributed on an "AS IS" BASIS,
//  WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
//  See the License for the specific language governing permissions and
//  limitations under the License.
