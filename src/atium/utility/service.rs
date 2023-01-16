use std::fs;
use log::{debug, error};
use uuid::Uuid;
use crate::atium::common::command_manager::CommandManager;
use crate::atium::common::error::AtiumError;

use crate::atium::utility::model::{InfoExtractorEngine, InfoExtractorRequest, InfoExtractorResponse, InfoExtractorResponseOutput, InfoFormat};

/// This service encapsulates the business logic to perform
/// a media file analysis and writes it to a requested output.
pub trait InfoExtractorService {
    /// Extracts infos from given input video file by using the requested engine.
    ///
    /// # Arguments
    ///
    /// * `request` - An instance of [`InfoExtractorRequest`] struct
    ///
    /// # Examples
    /// ```
    /// let selected_engine = InfoExtractorEngine::MediaInfo;
    /// let info_extractor_service = InfoExtractorBuilder::new(selected_engine).expect("error!");
    ///
    /// let request = InfoExtractorRequest {
    ///     input: "/path/to/video.mp4",
    /// }
    /// let info_response = info_extractor_service.get_info(request);
    /// ```
    fn get_info(&self, request: InfoExtractorRequest) -> Result<InfoExtractorResponse, AtiumError>;
}


/// This struct lets you build a new [`InfoExtractorService`] based on given engine
pub struct InfoExtractorBuilder {}

impl InfoExtractorBuilder {
    /// Creates a new instance of [`InfoExtractorService`] with the requested loaded engine.
    /// Current supported engines are:
    /// * `MediaInfo`
    ///
    /// # Arguments
    ///
    /// * `engine` - Any value of [`InfoExtractorEngine`] enum
    ///
    /// # Examples
    /// ```
    /// let selected_engine = InfoExtractorEngine::MediaInfo;
    /// let info_extractor_service = InfoExtractorBuilder::new(selected_engine).expect("error!");
    /// ```
    pub fn new(engine: InfoExtractorEngine) -> Result<Box<dyn InfoExtractorService>, AtiumError> {
        return match engine {
            InfoExtractorEngine::MediaInfo => {
                debug!("Creating a new MEDIAINFO service");
                let command_manager =
                    CommandManager::new("mediainfo".to_string(), vec!["--version"])
                        .expect("could not load command!");
                debug!("MEDIAINFO service created!");
                Ok(Box::new(MediaInfoExtractorService {
                    command_manager
                }))
            }
        }
    }
}

/// MediaInfo Engine Service for info extraction
pub struct MediaInfoExtractorService {
    command_manager: CommandManager
}

impl MediaInfoExtractorService {
    fn write_to_stdout(&self, output: std::process::Output) -> Result<(), AtiumError> {
        self.command_manager.print_command_output(output.stdout)
    }
    fn write_info_to_file(&self, output: std::process::Output, out_filepath: String, format: InfoFormat) -> Result<String, &'static str> {
        let ext = match format {
            InfoFormat::Json => ".json",
            InfoFormat::Html => ".html",
            InfoFormat::Xml => ".xml"
        };
        let mut id = out_filepath;
        if id.is_empty() {
            id = Uuid::new_v4().to_string();
        }
        let filename = id;
        let path = filename + ext;

        match fs::write(path.clone(), output.stdout) {
            Ok(_) => {
                debug!("Successfully wrote info to file");
                Ok(path)
            },
            Err(err) => {
                error!("Could not write to file: {}", err);
                Err("could not write to file!")
            }
        }
    }
    fn write_result(&self, execution_result: std::process::Output, output_file: Option<String>, format: InfoFormat) -> Result<InfoExtractorResponse, AtiumError> {
        return match output_file {
            None => self.write_to_stdout(execution_result)
                .map(|_| InfoExtractorResponse {
                    output: InfoExtractorResponseOutput {
                        file: None
                    }
                }),
            Some(output_filepath) => {
                self.write_info_to_file(execution_result, output_filepath, format)
                    .map(|output| InfoExtractorResponse {
                        output: InfoExtractorResponseOutput {
                            file: Some(output)
                        }
                    })
                    .map_err(|err_msg| AtiumError::IOError(err_msg.to_string()))
            }
        }
    }
}

impl InfoExtractorService for MediaInfoExtractorService {
    fn get_info(&self, request: InfoExtractorRequest) -> Result<InfoExtractorResponse, AtiumError> {
        let format = request.format.unwrap_or(InfoFormat::Json);
        let full = request.full.unwrap_or(true);

        let mut args: Vec<&str> = Vec::new();

        match format {
            InfoFormat::Json => {
                args.push("--output=JSON");
            }
            InfoFormat::Html => {
                args.push("--output=HTML");
            }
            InfoFormat::Xml => {
                args.push("--output=XML");
            }
        }

        if full {
            args.push("--full");
        }

        args.push(request.input.as_str());

        return match self.command_manager.execute_with_args(args) {
            Ok(execution_result) => {
                if !execution_result.status.success() {
                    // WARN: MEDIAINFO WRITES ERRORS TO STDOUT
                    self.command_manager.print_command_output(execution_result.stdout)?;
                    return Err(AtiumError::CommandError("Command execution returned ERROR status".to_string()))
                }

                self.write_result(execution_result, request.output_file, format)
            }
            Err(_) => Err(AtiumError::CommandError("Could not execute command".to_string()))
        }
    }
}