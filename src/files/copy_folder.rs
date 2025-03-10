use crate::common::delegate::UploadDelegateConfig;
use crate::common::drive_file;
use crate::common::hub_helper;
use crate::files;
use crate::files::copy;
use crate::files::list;
use crate::files::mkdir;
use crate::files::rename;
use crate::hub::Hub;
use std::error;
use std::fmt::Display;
use std::fmt::Formatter;

#[derive(Clone, Debug)]
pub struct Config {
    pub src_folder_id: String,
    pub to_folder_id: String,
}

pub async fn copy_folder(config: Config) -> Result<(), Error> {
    let hub = hub_helper::get_hub().await.map_err(Error::Hub)?;
    let delegate_config = UploadDelegateConfig::default();

    let src_folder = files::info::get_file(&hub, &config.src_folder_id)
        .await
        .map_err(Error::GetFile)?;

    err_if_not_directory(&src_folder)?;

    let to_parent = files::info::get_file(&hub, &config.to_folder_id)
        .await
        .map_err(Error::GetDestinationFolder)?;

    err_if_not_directory(&to_parent)?;

    let copy_config = CopyFolderConfig {
        src_folder_id: config.src_folder_id,
        to_folder_id: config.to_folder_id,
    };

    copy_folder_inner(&hub, delegate_config, &copy_config).await?;

    Ok(())
}

pub struct CopyFolderConfig {
    pub src_folder_id: String,
    pub to_folder_id: String,
}

pub async fn copy_folder_inner(
    hub: &Hub,
    delegate_config: UploadDelegateConfig,
    config: &CopyFolderConfig,
) -> Result<(), Error> {
    let list_children_config: list::ListFilesConfig = list::ListFilesConfig {
        query: list::ListQuery::FilesInFolder {
            folder_id: config.src_folder_id.clone(),
        },
        order_by: list::ListSortOrder::FolderModifiedName,
        max_files: 1000,
    };

    let children = list::list_files(hub, &list_children_config)
        .await
        .map_err(Error::ListFiles)?;

    for child in children {
        if drive_file::is_directory(&child) {
            let mkdir_config = mkdir::Config {
                id: None,
                name: child.name.ok_or(Error::FileWithoutId)?.clone(),
                parents: Some(vec![config.to_folder_id.clone()]),
                print_only_id: true,
            };
            let new_folder = mkdir::create_directory(hub, &mkdir_config, delegate_config.clone())
                .await
                .map_err(Error::MKDirError)?;
            let new_folder_config = CopyFolderConfig {
                src_folder_id: child.id.ok_or(Error::FileWithoutId)?,
                to_folder_id: new_folder.id.ok_or(Error::FileWithoutId)?,
            };
            Box::pin(copy_folder_inner(
                hub,
                delegate_config.clone(),
                &new_folder_config,
            ))
            .await?;
        } else {
            let copy_config = copy::CopyConfig {
                file_id: child.id.ok_or(Error::FileWithoutId)?,
                to_folder_id: config.to_folder_id.clone(),
            };
            let new_file = copy::copy_file(hub, delegate_config.clone(), &copy_config)
                .await
                .map_err(Error::Copy)?;

            let rename_config = rename::Config {
                file_id: new_file.id.ok_or(Error::FileWithoutId)?,
                name: child.name.ok_or(Error::FileWithoutName)?,
            };
            rename::rename(rename_config).await.map_err(Error::RenameError)?;
        }
    }

    Ok(())
}

impl Display for Error {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        match self {
            Error::Hub(err) => write!(f, "{}", err),
            Error::GetFile(err) => {
                write!(f, "Failed to get file: {}", err)
            }
            Error::GetDestinationFolder(err) => {
                write!(f, "Failed to get destination folder: {}", err)
            }
            Error::DestinationNotADirectory => {
                write!(f, "Can only copy to a directory")
            }
            Error::SourceIsADirectory => {
                write!(f, "Copy directories is not supported")
            }
            Error::ListFiles(err) => {
                write!(f, "Failed to list files: {}", err)
            }
            Error::Copy(err) => {
                write!(f, "Failed to move file: {}", err)
            }
            Error::FileWithoutId => {
                write!(f, "Found a file to copy with no id")
            }
            Error::FileWithoutName => {
                write!(f, "Found a file to copy with no name")
            }
            Error::MKDirError(err) => {
                write!(f, "Failed creating folder: {}", err)
            }
            Error::CopyFile(err) => {
                write!(f, "Faild copying file: {}", err)
            }
            Error::RenameError(err) => {
                write!(f, "Faild renaming file: {}", err)
            }
        }
    }
}

#[derive(Debug)]
pub enum Error {
    Hub(hub_helper::Error),
    GetFile(google_drive3::Error),
    GetDestinationFolder(google_drive3::Error),
    DestinationNotADirectory,
    SourceIsADirectory,
    ListFiles(list::Error),
    Copy(google_drive3::Error),
    FileWithoutId,
    FileWithoutName,
    MKDirError(google_drive3::Error),
    CopyFile(google_drive3::Error),
    RenameError(rename::Error),
}

impl error::Error for Error {}

fn err_if_not_directory(file: &google_drive3::api::File) -> Result<(), Error> {
    if !drive_file::is_directory(file) {
        Err(Error::DestinationNotADirectory)
    } else {
        Ok(())
    }
}
