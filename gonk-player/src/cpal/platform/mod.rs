#[doc(inline)]
pub use self::platform_impl::*;

macro_rules! impl_platform_host {
    ($($(#[cfg($feat: meta)])? $HostVariant:ident $host_mod:ident $host_name:literal),*) => {

        pub const ALL_HOSTS: &'static [HostId] = &[
            $(
                $(#[cfg($feat)])?
                HostId::$HostVariant,
            )*
        ];

        pub struct Host(HostInner);
        pub struct Device(DeviceInner);
        pub struct Devices(DevicesInner);

        // Streams cannot be `Send` or `Sync` if we plan to support Android's AAudio API. This is
        // because the stream API is not thread-safe, and the API prohibits calling certain
        // functions within the callback.
        //
        // TODO: Confirm this and add more specific detail and references.
        pub struct Stream(StreamInner, crate::cpal::platform::NotSendSyncAcrossAllPlatforms);



        pub struct SupportedInputConfigs(SupportedInputConfigsInner);



        pub struct SupportedOutputConfigs(SupportedOutputConfigsInner);


        #[derive(Copy, Clone, Debug, Eq, Hash, PartialEq)]
        pub enum HostId {
            $(
                $(#[cfg($feat)])?
                $HostVariant,
            )*
        }


        pub enum DeviceInner {
            $(
                $(#[cfg($feat)])?
                $HostVariant(crate::cpal::host::$host_mod::Device),
            )*
        }


        pub enum DevicesInner {
            $(
                $(#[cfg($feat)])?
                $HostVariant(crate::cpal::host::$host_mod::Devices),
            )*
        }


        pub enum HostInner {
            $(
                $(#[cfg($feat)])?
                $HostVariant(crate::cpal::host::$host_mod::Host),
            )*
        }


        pub enum StreamInner {
            $(
                $(#[cfg($feat)])?
                $HostVariant(crate::cpal::host::$host_mod::Stream),
            )*
        }

        enum SupportedInputConfigsInner {
            $(
                $(#[cfg($feat)])?
                $HostVariant(crate::cpal::host::$host_mod::SupportedInputConfigs),
            )*
        }

        enum SupportedOutputConfigsInner {
            $(
                $(#[cfg($feat)])?
                $HostVariant(crate::cpal::host::$host_mod::SupportedOutputConfigs),
            )*
        }

        impl HostId {
            pub fn name(&self) -> &'static str {
                match self {
                    $(
                        $(#[cfg($feat)])?
                        HostId::$HostVariant => $host_name,
                    )*
                }
            }
        }

        impl Devices {


            pub fn as_inner(&self) -> &DevicesInner {
                &self.0
            }



            pub fn as_inner_mut(&mut self) -> &mut DevicesInner {
                &mut self.0
            }


            pub fn into_inner(self) -> DevicesInner {
                self.0
            }
        }

        impl Device {


            pub fn as_inner(&self) -> &DeviceInner {
                &self.0
            }



            pub fn as_inner_mut(&mut self) -> &mut DeviceInner {
                &mut self.0
            }


            pub fn into_inner(self) -> DeviceInner {
                self.0
            }
        }

        impl Host {

            pub fn id(&self) -> HostId {
                match self.0 {
                    $(
                        $(#[cfg($feat)])?
                        HostInner::$HostVariant(_) => HostId::$HostVariant,
                    )*
                }
            }



            pub fn as_inner(&self) -> &HostInner {
                &self.0
            }



            pub fn as_inner_mut(&mut self) -> &mut HostInner {
                &mut self.0
            }


            pub fn into_inner(self) -> HostInner {
                self.0
            }
        }

        impl Stream {


            pub fn as_inner(&self) -> &StreamInner {
                &self.0
            }



            pub fn as_inner_mut(&mut self) -> &mut StreamInner {
                &mut self.0
            }


            pub fn into_inner(self) -> StreamInner {
                self.0
            }
        }

        impl Iterator for Devices {
            type Item = Device;

            fn next(&mut self) -> Option<Self::Item> {
                match self.0 {
                    $(
                        $(#[cfg($feat)])?
                        DevicesInner::$HostVariant(ref mut d) => {
                            d.next().map(DeviceInner::$HostVariant).map(Device::from)
                        }
                    )*
                }
            }

            fn size_hint(&self) -> (usize, Option<usize>) {
                match self.0 {
                    $(
                        $(#[cfg($feat)])?
                        DevicesInner::$HostVariant(ref d) => d.size_hint(),
                    )*
                }
            }
        }

        impl Iterator for SupportedInputConfigs {
            type Item = crate::cpal::SupportedStreamConfigRange;

            fn next(&mut self) -> Option<Self::Item> {
                match self.0 {
                    $(
                        $(#[cfg($feat)])?
                        SupportedInputConfigsInner::$HostVariant(ref mut s) => s.next(),
                    )*
                }
            }

            fn size_hint(&self) -> (usize, Option<usize>) {
                match self.0 {
                    $(
                        $(#[cfg($feat)])?
                        SupportedInputConfigsInner::$HostVariant(ref d) => d.size_hint(),
                    )*
                }
            }
        }

        impl Iterator for SupportedOutputConfigs {
            type Item = crate::cpal::SupportedStreamConfigRange;

            fn next(&mut self) -> Option<Self::Item> {
                match self.0 {
                    $(
                        $(#[cfg($feat)])?
                        SupportedOutputConfigsInner::$HostVariant(ref mut s) => s.next(),
                    )*
                }
            }

            fn size_hint(&self) -> (usize, Option<usize>) {
                match self.0 {
                    $(
                        $(#[cfg($feat)])?
                        SupportedOutputConfigsInner::$HostVariant(ref d) => d.size_hint(),
                    )*
                }
            }
        }

        impl crate::cpal::traits::DeviceTrait for Device {
            type SupportedInputConfigs = SupportedInputConfigs;
            type SupportedOutputConfigs = SupportedOutputConfigs;
            type Stream = Stream;

            fn name(&self) -> Result<String, crate::cpal::DeviceNameError> {
                match self.0 {
                    $(
                        $(#[cfg($feat)])?
                        DeviceInner::$HostVariant(ref d) => d.name(),
                    )*
                }
            }

            fn supported_input_configs(&self) -> Result<Self::SupportedInputConfigs, crate::cpal::SupportedStreamConfigsError> {
                match self.0 {
                    $(
                        $(#[cfg($feat)])?
                        DeviceInner::$HostVariant(ref d) => {
                            d.supported_input_configs()
                                .map(SupportedInputConfigsInner::$HostVariant)
                                .map(SupportedInputConfigs)
                        }
                    )*
                }
            }

            fn supported_output_configs(&self) -> Result<Self::SupportedOutputConfigs, crate::cpal::SupportedStreamConfigsError> {
                match self.0 {
                    $(
                        $(#[cfg($feat)])?
                        DeviceInner::$HostVariant(ref d) => {
                            d.supported_output_configs()
                                .map(SupportedOutputConfigsInner::$HostVariant)
                                .map(SupportedOutputConfigs)
                        }
                    )*
                }
            }

            fn default_input_config(&self) -> Result<crate::cpal::SupportedStreamConfig, crate::cpal::DefaultStreamConfigError> {
                match self.0 {
                    $(
                        $(#[cfg($feat)])?
                        DeviceInner::$HostVariant(ref d) => d.default_input_config(),
                    )*
                }
            }

            fn default_output_config(&self) -> Result<crate::cpal::SupportedStreamConfig, crate::cpal::DefaultStreamConfigError> {
                match self.0 {
                    $(
                        $(#[cfg($feat)])?
                        DeviceInner::$HostVariant(ref d) => d.default_output_config(),
                    )*
                }
            }

            fn build_input_stream_raw<D, E>(
                &self,
                config: &crate::cpal::StreamConfig,
                sample_format: crate::cpal::SampleFormat,
                data_callback: D,
                error_callback: E,
            ) -> Result<Self::Stream, crate::cpal::BuildStreamError>
            where
                D: FnMut(&crate::cpal::Data, &crate::cpal::InputCallbackInfo) + Send + 'static,
                E: FnMut(crate::cpal::StreamError) + Send + 'static,
            {
                match self.0 {
                    $(
                        $(#[cfg($feat)])?
                        DeviceInner::$HostVariant(ref d) => d
                            .build_input_stream_raw(
                                config,
                                sample_format,
                                data_callback,
                                error_callback,
                            )
                            .map(StreamInner::$HostVariant)
                            .map(Stream::from),
                    )*
                }
            }

            fn build_output_stream_raw<D, E>(
                &self,
                config: &crate::cpal::StreamConfig,
                sample_format: crate::cpal::SampleFormat,
                data_callback: D,
                error_callback: E,
            ) -> Result<Self::Stream, crate::cpal::BuildStreamError>
            where
                D: FnMut(&mut crate::cpal::Data, &crate::cpal::OutputCallbackInfo) + Send + 'static,
                E: FnMut(crate::cpal::StreamError) + Send + 'static,
            {
                match self.0 {
                    $(
                        $(#[cfg($feat)])?
                        DeviceInner::$HostVariant(ref d) => d
                            .build_output_stream_raw(
                                config,
                                sample_format,
                                data_callback,
                                error_callback,
                            )
                            .map(StreamInner::$HostVariant)
                            .map(Stream::from),
                    )*
                }
            }
        }

        impl crate::cpal::traits::HostTrait for Host {
            type Devices = Devices;
            type Device = Device;

            fn is_available() -> bool {
                $(
                    $(#[cfg($feat)])?
                    if crate::cpal::host::$host_mod::Host::is_available() { return true; }
                )*
                false
            }

            fn devices(&self) -> Result<Self::Devices, crate::cpal::DevicesError> {
                match self.0 {
                    $(
                        $(#[cfg($feat)])?
                        HostInner::$HostVariant(ref h) => {
                            h.devices().map(DevicesInner::$HostVariant).map(Devices::from)
                        }
                    )*
                }
            }

            fn default_input_device(&self) -> Option<Self::Device> {
                match self.0 {
                    $(
                        $(#[cfg($feat)])?
                        HostInner::$HostVariant(ref h) => {
                            h.default_input_device().map(DeviceInner::$HostVariant).map(Device::from)
                        }
                    )*
                }
            }

            fn default_output_device(&self) -> Option<Self::Device> {
                match self.0 {
                    $(
                        $(#[cfg($feat)])?
                        HostInner::$HostVariant(ref h) => {
                            h.default_output_device().map(DeviceInner::$HostVariant).map(Device::from)
                        }
                    )*
                }
            }
        }

        impl crate::cpal::traits::StreamTrait for Stream {
            fn play(&self) -> Result<(), crate::cpal::PlayStreamError> {
                match self.0 {
                    $(
                        $(#[cfg($feat)])?
                        StreamInner::$HostVariant(ref s) => {
                            s.play()
                        }
                    )*
                }
            }

            fn pause(&self) -> Result<(), crate::cpal::PauseStreamError> {
                match self.0 {
                    $(
                        $(#[cfg($feat)])?
                        StreamInner::$HostVariant(ref s) => {
                            s.pause()
                        }
                    )*
                }
            }
        }

        impl From<DeviceInner> for Device {
            fn from(d: DeviceInner) -> Self {
                Device(d)
            }
        }

        impl From<DevicesInner> for Devices {
            fn from(d: DevicesInner) -> Self {
                Devices(d)
            }
        }

        impl From<HostInner> for Host {
            fn from(h: HostInner) -> Self {
                Host(h)
            }
        }

        impl From<StreamInner> for Stream {
            fn from(s: StreamInner) -> Self {
                Stream(s, Default::default())
            }
        }

        $(
            $(#[cfg($feat)])?
            impl From<crate::cpal::host::$host_mod::Device> for Device {
                fn from(h: crate::cpal::host::$host_mod::Device) -> Self {
                    DeviceInner::$HostVariant(h).into()
                }
            }

            $(#[cfg($feat)])?
            impl From<crate::cpal::host::$host_mod::Devices> for Devices {
                fn from(h: crate::cpal::host::$host_mod::Devices) -> Self {
                    DevicesInner::$HostVariant(h).into()
                }
            }

            $(#[cfg($feat)])?
            impl From<crate::cpal::host::$host_mod::Host> for Host {
                fn from(h: crate::cpal::host::$host_mod::Host) -> Self {
                    HostInner::$HostVariant(h).into()
                }
            }

            $(#[cfg($feat)])?
            impl From<crate::cpal::host::$host_mod::Stream> for Stream {
                fn from(h: crate::cpal::host::$host_mod::Stream) -> Self {
                    StreamInner::$HostVariant(h).into()
                }
            }
        )*


        pub fn available_hosts() -> Vec<HostId> {
            let mut host_ids = vec![];
            $(
                $(#[cfg($feat)])?
                if <crate::cpal::host::$host_mod::Host as crate::cpal::traits::HostTrait>::is_available() {
                    host_ids.push(HostId::$HostVariant);
                }
            )*
            host_ids
        }


        pub fn host_from_id(id: HostId) -> Result<Host, crate::cpal::HostUnavailable> {
            match id {
                $(
                    $(#[cfg($feat)])?
                    HostId::$HostVariant => {
                        crate::cpal::host::$host_mod::Host::new()
                            .map(HostInner::$HostVariant)
                            .map(Host::from)
                    }
                )*
            }
        }
    };
}

// TODO: Add pulseaudio and jack here eventually.
#[cfg(any(target_os = "linux", target_os = "dragonfly", target_os = "freebsd"))]
mod platform_impl {
    pub use crate::cpal::host::alsa::{
        Device as AlsaDevice, Devices as AlsaDevices, Host as AlsaHost, Stream as AlsaStream,
        SupportedInputConfigs as AlsaSupportedInputConfigs,
        SupportedOutputConfigs as AlsaSupportedOutputConfigs,
    };
    #[cfg(feature = "jack")]
    pub use crate::cpal::host::jack::{
        Device as JackDevice, Devices as JackDevices, Host as JackHost, Stream as JackStream,
        SupportedInputConfigs as JackSupportedInputConfigs,
        SupportedOutputConfigs as JackSupportedOutputConfigs,
    };

    impl_platform_host!(#[cfg(feature = "jack")] Jack jack "JACK", Alsa alsa "ALSA");

    pub fn default_host() -> Host {
        AlsaHost::new()
            .expect("the default host should always be available")
            .into()
    }
}

#[cfg(any(target_os = "macos", target_os = "ios"))]
mod platform_impl {
    pub use crate::cpal::host::coreaudio::{
        Device as CoreAudioDevice, Devices as CoreAudioDevices, Host as CoreAudioHost,
        Stream as CoreAudioStream, SupportedInputConfigs as CoreAudioSupportedInputConfigs,
        SupportedOutputConfigs as CoreAudioSupportedOutputConfigs,
    };

    impl_platform_host!(CoreAudio coreaudio "CoreAudio");

    pub fn default_host() -> Host {
        CoreAudioHost::new()
            .expect("the default host should always be available")
            .into()
    }
}

#[cfg(target_os = "emscripten")]
mod platform_impl {
    pub use crate::cpal::host::emscripten::{
        Device as EmscriptenDevice, Devices as EmscriptenDevices, Host as EmscriptenHost,
        Stream as EmscriptenStream, SupportedInputConfigs as EmscriptenSupportedInputConfigs,
        SupportedOutputConfigs as EmscriptenSupportedOutputConfigs,
    };

    impl_platform_host!(Emscripten emscripten "Emscripten");

    pub fn default_host() -> Host {
        EmscriptenHost::new()
            .expect("the default host should always be available")
            .into()
    }
}

#[cfg(all(target_arch = "wasm32", feature = "wasm-bindgen"))]
mod platform_impl {
    pub use crate::cpal::host::webaudio::{
        Device as WebAudioDevice, Devices as WebAudioDevices, Host as WebAudioHost,
        Stream as WebAudioStream, SupportedInputConfigs as WebAudioSupportedInputConfigs,
        SupportedOutputConfigs as WebAudioSupportedOutputConfigs,
    };

    impl_platform_host!(WebAudio webaudio "WebAudio");

    pub fn default_host() -> Host {
        WebAudioHost::new()
            .expect("the default host should always be available")
            .into()
    }
}

#[cfg(windows)]
mod platform_impl {
    #[cfg(feature = "asio")]
    pub use crate::cpal::host::asio::{
        Device as AsioDevice, Devices as AsioDevices, Host as AsioHost, Stream as AsioStream,
        SupportedInputConfigs as AsioSupportedInputConfigs,
        SupportedOutputConfigs as AsioSupportedOutputConfigs,
    };
    pub use crate::cpal::host::wasapi::{
        Device as WasapiDevice, Devices as WasapiDevices, Host as WasapiHost,
        Stream as WasapiStream, SupportedInputConfigs as WasapiSupportedInputConfigs,
        SupportedOutputConfigs as WasapiSupportedOutputConfigs,
    };

    impl_platform_host!(#[cfg(feature = "asio")] Asio asio "ASIO", Wasapi wasapi "WASAPI");

    pub fn default_host() -> Host {
        WasapiHost::new()
            .expect("the default host should always be available")
            .into()
    }
}

#[cfg(target_os = "android")]
mod platform_impl {
    pub use crate::cpal::host::oboe::{
        Device as OboeDevice, Devices as OboeDevices, Host as OboeHost, Stream as OboeStream,
        SupportedInputConfigs as OboeSupportedInputConfigs,
        SupportedOutputConfigs as OboeSupportedOutputConfigs,
    };

    impl_platform_host!(Oboe oboe "Oboe");

    pub fn default_host() -> Host {
        OboeHost::new()
            .expect("the default host should always be available")
            .into()
    }
}

#[cfg(not(any(
    windows,
    target_os = "linux",
    target_os = "dragonfly",
    target_os = "freebsd",
    target_os = "macos",
    target_os = "ios",
    target_os = "emscripten",
    target_os = "android",
    all(target_arch = "wasm32", feature = "wasm-bindgen"),
)))]
mod platform_impl {
    pub use crate::cpal::host::null::{
        Device as NullDevice, Devices as NullDevices, Host as NullHost,
        SupportedInputConfigs as NullSupportedInputConfigs,
        SupportedOutputConfigs as NullSupportedOutputConfigs,
    };

    impl_platform_host!(Null null "Null");

    pub fn default_host() -> Host {
        NullHost::new()
            .expect("the default host should always be available")
            .into()
    }
}

// The following zero-sized types are for applying Send/Sync restrictions to ensure
// consistent behaviour across different platforms. These verbosely named types are used
// (rather than using the markers directly) in the hope of making the compile errors
// slightly more helpful.
//
// TODO: Remove these in favour of using negative trait bounds if they stabilise.

// A marker used to remove the `Send` and `Sync` traits.
struct NotSendSyncAcrossAllPlatforms(std::marker::PhantomData<*mut ()>);

impl Default for NotSendSyncAcrossAllPlatforms {
    fn default() -> Self {
        NotSendSyncAcrossAllPlatforms(std::marker::PhantomData)
    }
}
