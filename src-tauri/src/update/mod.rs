//! In-app updates: checks the release file on the update server, downloads the installer, checks
//! its signature against the public key built into Codestamp and runs it in update mode. A .deb
//! install only learns about the new version; apt installs it.

pub mod commands;
mod models;
mod route;
mod settings;
