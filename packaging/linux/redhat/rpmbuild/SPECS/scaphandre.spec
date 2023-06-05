Name:           scaphandre
Version:        CHANGEME
Release:        1%{?dist}
Summary:        Power usage / Electricity / Energy monitoring agent 

License:        Apache-2.0 
URL:            https://github.com/hubblo-org/scaphandre
Source0:        %{name}-%{version}.tar.gz
#Source0 will be github.com url for tar gz of source

BuildRequires:  rust,cargo,openssl-devel,systemd-rpm-macros 
#Requires:

%global debug_package %{nil}

%description

%prep
%autosetup

%build
cargo build --release

%pre

%install
#rm -rf $RPM_BUILD_ROOT
mkdir -p $RPM_BUILD_ROOT/%{_bindir}/
cp target/release/scaphandre $RPM_BUILD_ROOT/%{_bindir}/
chmod +x $RPM_BUILD_ROOT/%{_bindir}/scaphandre
mkdir -p $RPM_BUILD_ROOT/lib/systemd/system
mkdir -p $RPM_BUILD_ROOT/etc/scaphandre
echo "SCAPHANDRE_ARGS=prometheus" > $RPM_BUILD_ROOT/etc/scaphandre/default
mkdir -p $RPM_BUILD_ROOT/lib/systemd/system
cp packaging/linux/redhat/scaphandre.service $RPM_BUILD_ROOT/lib/systemd/system/scaphandre.service

%post
%systemd_post scaphandre.service

%preun
%systemd_preun scaphandre.service

%postun
%systemd_postun_with_restart scaphandre.service

%clean
#rm -rf $RPM_BUILD_ROOT

%files
#%doc README.md
%{_bindir}/scaphandre
/lib/systemd/system/scaphandre.service
/etc/scaphandre/default

#%license LICENSE

%changelog