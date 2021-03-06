use actix_multipart::{Field, Multipart};
use actix_web::{error, web, Error};
use futures::StreamExt;
use std::io::Write;
use std::str;

use crate::models::Project;

#[derive(Debug, Clone)]
pub struct UploadedFile {
    pub name: String,
    pub path: String,
}
impl UploadedFile {
    fn new(filename: &str, files_path: &str) -> UploadedFile {
        UploadedFile {
            name: filename.to_string(),
            path: format!("{}/{}", files_path, filename),
        }
    }
}

pub async fn split_payload(payload: &mut Multipart) -> Result<Project, Error> {
    // get the directory for static files from env variables
    let files_path = std::env::var("DIRECTORY.STATIC_FILES")
        .expect("DIRECTORY.STATIC_FILES must be set in the .env variables");
    let mut files: Vec<String> = Vec::new();

    /* create a default project mostly empty values */
    let mut project: Project = Default::default();
    let mut current_images_vec: Vec<String>=Vec::new();
    // iterate over all formdata fields
    while let Some(item) = payload.next().await {
        let mut field: Field = item.expect(" split_payload err");
        let content_type = field
            .content_disposition()
            .ok_or_else(|| error::ParseError::Incomplete)?;
        let name = content_type // get the fields name
            .get_name()
            .ok_or_else(|| error::ParseError::Incomplete)?;
        if name != "images" {
            // if the field name is not images try to handle it as textfield
            while let Some(chunk) = field.next().await {
                let data = chunk.expect("split_payload err chunk");

                if let Ok(s) = str::from_utf8(&data) {
                    let data_string = s.to_string();
                    // match the field names with all the valid fields and hydrate the project with it
                    match name {
                        "title" => project.name = data_string,
                        "description" => project.description = data_string,
                        "homepage" => project.homepage = data_string,
                        "repository" => project.repository = data_string,
                        "priority" => {
                            project.priority = Some(data_string.parse().expect("not a number"))
                        }
                        "technologies" => {
                            /* get an array of tech */
                            let technology_vec: Vec<String> = data_string
                                .split(',')
                                .map(|item| String::from(item.trim()))
                                .filter(|item| item!="") // filter out empty strings
                                .collect();
                            project.technologies = Some(technology_vec);
                        }
                        "currentimages" => {
                            /* get an array of tech */
                            current_images_vec = data_string
                                .split(',')
                                .map(|item| String::from(item.trim()))
                                .filter(|item| item!="") // filter out empty strings
                                .collect();
                            
                            //project.images = Some(current_images_vec);
                        }
                        _ => println!("invalid field found"),
                    };
                };
            }
        } else {
            // if the field name is images we might want to handle it as file
            match content_type.get_filename() {
                // check if file name is provided if so handle it as file
                Some(filename) => {
                    if filename != "" {
                        // empty string seems be a filename for rust but not for me
                        println!("filename {}", filename);
                        let file = UploadedFile::new(filename, &files_path); // create new UploadedFiles
                        let file_path = file.path.clone();
                        let mut f = web::block(move || std::fs::File::create(&file_path)).await?;
                        while let Some(chunk) = field.next().await {
                            let data = chunk.unwrap();
                            f = web::block(move || f.write_all(&data).map(|_| f)).await?
                        }
                        files.push(file.name); // form only needs name
                    }
                }
                None => {
                    //println!("file none");
                }
            }
        }
    }
    // currentimages only exist in the edit project form but didnt feel like creating another function
    if current_images_vec.len()>0 {
        if files.len()>0 {
            // if both vectors contain items merge them otherwise just assign files
            current_images_vec.extend_from_slice(&files);
            project.images=Some(current_images_vec);
        }else {
            project.images=Some(current_images_vec);
        }
    }else {
        project.images = Some(files);
    }
    Ok(project)
}
