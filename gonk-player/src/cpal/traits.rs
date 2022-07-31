use crate::cpal::{
    BuildStreamError, Data, DefaultStreamConfigError, DeviceNameError, DevicesError,
    InputCallbackInfo, InputDevices, OutputCallbackInfo, OutputDevices, PauseStreamError,
    PlayStreamError, Sample, SampleFormat, StreamConfig, StreamError, SupportedStreamConfig,
    SupportedStreamConfigRange, SupportedStreamConfigsError,
};

pub trait HostTrait {
    type Devices: Iterator<Item = Self::Device>;

    type Device: DeviceTrait;

    fn is_available() -> bool;

    fn devices(&self) -> Result<Self::Devices, DevicesError>;

    fn default_input_device(&self) -> Option<Self::Device>;

    fn default_output_device(&self) -> Option<Self::Device>;

    fn input_devices(&self) -> Result<InputDevices<Self::Devices>, DevicesError> {
        fn supports_input<D: DeviceTrait>(device: &D) -> bool {
            device
                .supported_input_configs()
                .map(|mut iter| iter.next().is_some())
                .unwrap_or(false)
        }
        Ok(self.devices()?.filter(supports_input::<Self::Device>))
    }

    fn output_devices(&self) -> Result<OutputDevices<Self::Devices>, DevicesError> {
        fn supports_output<D: DeviceTrait>(device: &D) -> bool {
            device
                .supported_output_configs()
                .map(|mut iter| iter.next().is_some())
                .unwrap_or(false)
        }
        Ok(self.devices()?.filter(supports_output::<Self::Device>))
    }
}

pub trait DeviceTrait {
    type SupportedInputConfigs: Iterator<Item = SupportedStreamConfigRange>;

    type SupportedOutputConfigs: Iterator<Item = SupportedStreamConfigRange>;

    type Stream: StreamTrait;

    fn name(&self) -> Result<String, DeviceNameError>;

    fn supported_input_configs(
        &self,
    ) -> Result<Self::SupportedInputConfigs, SupportedStreamConfigsError>;

    fn supported_output_configs(
        &self,
    ) -> Result<Self::SupportedOutputConfigs, SupportedStreamConfigsError>;

    fn default_input_config(&self) -> Result<SupportedStreamConfig, DefaultStreamConfigError>;

    fn default_output_config(&self) -> Result<SupportedStreamConfig, DefaultStreamConfigError>;

    fn build_input_stream<T, D, E>(
        &self,
        config: &StreamConfig,
        mut data_callback: D,
        error_callback: E,
    ) -> Result<Self::Stream, BuildStreamError>
    where
        T: Sample,
        D: FnMut(&[T], &InputCallbackInfo) + Send + 'static,
        E: FnMut(StreamError) + Send + 'static,
    {
        self.build_input_stream_raw(
            config,
            T::FORMAT,
            move |data, info| {
                data_callback(
                    data.as_slice()
                        .expect("host supplied incorrect sample type"),
                    info,
                )
            },
            error_callback,
        )
    }

    fn build_output_stream<T, D, E>(
        &self,
        config: &StreamConfig,
        mut data_callback: D,
        error_callback: E,
    ) -> Result<Self::Stream, BuildStreamError>
    where
        T: Sample,
        D: FnMut(&mut [T], &OutputCallbackInfo) + Send + 'static,
        E: FnMut(StreamError) + Send + 'static,
    {
        self.build_output_stream_raw(
            config,
            T::FORMAT,
            move |data, info| {
                data_callback(
                    data.as_slice_mut()
                        .expect("host supplied incorrect sample type"),
                    info,
                )
            },
            error_callback,
        )
    }

    fn build_input_stream_raw<D, E>(
        &self,
        config: &StreamConfig,
        sample_format: SampleFormat,
        data_callback: D,
        error_callback: E,
    ) -> Result<Self::Stream, BuildStreamError>
    where
        D: FnMut(&Data, &InputCallbackInfo) + Send + 'static,
        E: FnMut(StreamError) + Send + 'static;

    fn build_output_stream_raw<D, E>(
        &self,
        config: &StreamConfig,
        sample_format: SampleFormat,
        data_callback: D,
        error_callback: E,
    ) -> Result<Self::Stream, BuildStreamError>
    where
        D: FnMut(&mut Data, &OutputCallbackInfo) + Send + 'static,
        E: FnMut(StreamError) + Send + 'static;
}

pub trait StreamTrait {
    fn play(&self) -> Result<(), PlayStreamError>;

    fn pause(&self) -> Result<(), PauseStreamError>;
}
