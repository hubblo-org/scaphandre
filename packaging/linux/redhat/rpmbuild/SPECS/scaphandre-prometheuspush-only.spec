Name:           scaphandre-prometheuspush
Version:        CHANGEME
Release:        1%{?dist}
Summary:        Power usage / Electricity / Energy monitoring agent 

License:        Apache-2.0 
URL:            https://github.com/hubblo-org/scaphandre
Source0:        %{name}-%{version}.tar.gz
#Source0 will be github.com url for tar gz of source

BuildRequires:  rust,cargo,systemd-rpm-macros
#Requires: 

%global debug_package %{nil}

%description

%prep
%autosetup

%build
cargo build --release --no-default-features --features json,prometheuspush

%pre

%install
#rm -rf $RPM_BUILD_ROOT
mkdir -p $RPM_BUILD_ROOT/%{_bindir}/
cp target/release/scaphandre $RPM_BUILD_ROOT/%{_bindir}/scaphandre-prometheuspush
chmod +x $RPM_BUILD_ROOT/%{_bindir}/scaphandre-prometheuspush
mkdir -p $RPM_BUILD_ROOT/lib/systemd/system
mkdir -p $RPM_BUILD_ROOT/etc/scaphandre
echo 'SCAPHANDRE_ARGS="prometheus-push -H localhost -S http"' > $RPM_BUILD_ROOT/etc/scaphandre/prometheuspush
mkdir -p $RPM_BUILD_ROOT/lib/systemd/system
cp packaging/linux/redhat/scaphandre-prometheuspush.service $RPM_BUILD_ROOT/lib/systemd/system/scaphandre-prometheuspush.service

%post
%systemd_post scaphandre-prometheuspush.service

%preun
%systemd_preun scaphandre-prometheuspush.service

%postun
%systemd_postun_with_restart scaphandre-prometheuspush.service

%clean
#rm -rf $RPM_BUILD_ROOT

%files
#%doc README.md
%{_bindir}/scaphandre-prometheuspush
/lib/systemd/system/scaphandre-prometheuspush.service
/etc/scaphandre/prometheuspush

#%license LICENSE

%changelog
