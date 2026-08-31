use crate::error::Result;

pub fn configure(options: crate::package_profile::ConfigureOptions) -> Result<()> {
    crate::package_profile::configure(options)
}
