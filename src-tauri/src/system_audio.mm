#import <CoreAudio/AudioHardware.h>
#import <CoreAudio/AudioHardwareTapping.h>
#import <CoreAudio/CATapDescription.h>
#import <Foundation/Foundation.h>

#include <vector>
#include <string.h>

typedef void (*system_audio_callback)(const float*, size_t, double, uint32_t, void*);

@interface SystemAudioTap : NSObject
@property(nonatomic, assign) AudioObjectID tapID;
@property(nonatomic, assign) AudioDeviceID deviceID;
@property(nonatomic, assign) AudioDeviceIOProcID procID;
@property(nonatomic, assign) system_audio_callback callback;
@property(nonatomic, assign) void* userData;
@property(nonatomic, assign) double sampleRate;
@property(nonatomic, assign, getter=isRunning) BOOL running;
@end

@implementation SystemAudioTap
@end

static void set_error(NSString* description, char** errorOut) {
    if (errorOut == nullptr) {
        return;
    }

    if (*errorOut != nullptr) {
        free(*errorOut);
        *errorOut = nullptr;
    }

    if (!description) {
        return;
    }

    const char* utf8 = [description UTF8String];
    if (utf8) {
        *errorOut = strdup(utf8);
    }
}

static OSStatus tap_io_proc(
    AudioObjectID inDevice,
    const AudioTimeStamp* inNow,
    const AudioBufferList* inInputData,
    const AudioTimeStamp* inInputTime,
    AudioBufferList* outOutputData,
    const AudioTimeStamp* inOutputTime,
    void* __nullable inClientData
) {
    (void)inDevice;
    (void)inNow;
    (void)inInputTime;
    (void)outOutputData;
    (void)inOutputTime;

    SystemAudioTap* tap = (__bridge SystemAudioTap*)inClientData;
    if (tap == nil || tap.callback == nullptr || inInputData == nullptr) {
        return noErr;
    }

    const UInt32 bufferCount = inInputData->mNumberBuffers;
    if (bufferCount == 0) {
        return noErr;
    }

    UInt32 channelsPerFrame = 0;
    UInt32 framesPerBuffer = 0;

    for (UInt32 index = 0; index < bufferCount; ++index) {
        const AudioBuffer buffer = inInputData->mBuffers[index];
        if (buffer.mData == nullptr || buffer.mDataByteSize == 0) {
            continue;
        }

        const UInt32 channels = buffer.mNumberChannels;
        const UInt32 frames = buffer.mDataByteSize / (sizeof(Float32) * channels);

        if (channelsPerFrame == 0) {
            channelsPerFrame = channels;
            framesPerBuffer = frames;
        }
    }

    if (channelsPerFrame == 0 || framesPerBuffer == 0) {
        return noErr;
    }

    std::vector<float> mono(framesPerBuffer, 0.f);
    float channelCount = 0.f;

    for (UInt32 index = 0; index < bufferCount; ++index) {
        const AudioBuffer buffer = inInputData->mBuffers[index];
        if (buffer.mData == nullptr || buffer.mDataByteSize == 0) {
            continue;
        }

        const UInt32 channels = buffer.mNumberChannels;
        const UInt32 frames = buffer.mDataByteSize / (sizeof(Float32) * channels);
        const Float32* data = static_cast<const Float32*>(buffer.mData);

        channelCount += static_cast<float>(channels);

        for (UInt32 frame = 0; frame < frames; ++frame) {
            float sum = 0.f;
            for (UInt32 channel = 0; channel < channels; ++channel) {
                sum += data[frame * channels + channel];
            }
            mono[frame] += sum;
        }
    }

    if (channelCount <= 0.f) {
        return noErr;
    }

    const float normalization = 1.f / channelCount;
    for (float& sample : mono) {
        sample *= normalization;
    }

    tap.callback(mono.data(), mono.size(), tap.sampleRate, 1, tap.userData);
    return noErr;
}

extern "C" void* system_audio_tap_start(
    double preferred_sample_rate,
    system_audio_callback callback,
    void* userData,
    char** errorOut,
    double* actual_sample_rate,
    uint32_t* actual_channels
) {
    @autoreleasepool {
        if (@available(macOS 14.2, *)) {
            SystemAudioTap* tap = [[SystemAudioTap alloc] init];
            tap.callback = callback;
            tap.userData = userData;
            tap.running = NO;

            NSArray<NSNumber*>* excluded = @[];
            CATapDescription* description = [[CATapDescription alloc] initStereoGlobalTapButExcludeProcesses:excluded];
            if (description == nil) {
                set_error(@"Failed to create tap description", errorOut);
                return nullptr;
            }

            [description setExclusive:YES];
            [description setPrivate:YES];
            [description setMuteBehavior:CATapUnmuted];
            [description setName:@"MemoBreezeSystemTap"];

            AudioObjectID tapID = kAudioObjectUnknown;
            OSStatus status = AudioHardwareCreateProcessTap(description, &tapID);
            if (status != noErr) {
                set_error([NSString stringWithFormat:@"Failed to create process tap (%d)", status], errorOut);
                return nullptr;
            }

            tap.tapID = tapID;

            AudioStreamBasicDescription format;
            memset(&format, 0, sizeof(AudioStreamBasicDescription));
            UInt32 formatSize = sizeof(AudioStreamBasicDescription);
            AudioObjectPropertyAddress formatAddress = {
                .mSelector = kAudioTapPropertyFormat,
                .mScope = kAudioObjectPropertyScopeGlobal,
                .mElement = kAudioObjectPropertyElementMain,
            };

            status = AudioObjectGetPropertyData(tapID, &formatAddress, 0, nullptr, &formatSize, &format);
            if (status != noErr) {
                AudioHardwareDestroyProcessTap(tapID);
                set_error([NSString stringWithFormat:@"Failed to read tap format (%d)", status], errorOut);
                return nullptr;
            }

            tap.sampleRate = format.mSampleRate > 0.0 ? format.mSampleRate : (preferred_sample_rate > 0.0 ? preferred_sample_rate : 44100.0);

            if (actual_sample_rate != nullptr) {
                *actual_sample_rate = tap.sampleRate;
            }

            if (actual_channels != nullptr) {
                *actual_channels = format.mChannelsPerFrame > 0 ? format.mChannelsPerFrame : 2;
            }

            NSString* tapUUID = [[[description UUID] UUIDString] copy];

            NSDictionary* tapEntry = @{
                @kAudioSubTapUIDKey : tapUUID,
                @kAudioSubTapDriftCompensationKey : @YES,
            };

            NSDictionary* aggregateDescription = @{
                @kAudioAggregateDeviceNameKey : @"MemoBreezeSystemAudio",
                @kAudioAggregateDeviceUIDKey : @"com.memobreeze.systemaudio",
                @kAudioAggregateDeviceTapListKey : @[tapEntry],
                @kAudioAggregateDeviceTapAutoStartKey : @NO,
                @kAudioAggregateDeviceIsPrivateKey : @YES,
            };

            AudioDeviceID deviceID = kAudioObjectUnknown;
            status = AudioHardwareCreateAggregateDevice((__bridge CFDictionaryRef)aggregateDescription, &deviceID);
            if (status != noErr) {
                AudioHardwareDestroyProcessTap(tapID);
                set_error([NSString stringWithFormat:@"Failed to create aggregate device (%d)", status], errorOut);
                return nullptr;
            }

            tap.deviceID = deviceID;

            if (preferred_sample_rate > 0.0) {
                Float64 rate = preferred_sample_rate;
                AudioObjectPropertyAddress rateAddress = {
                    .mSelector = kAudioDevicePropertyNominalSampleRate,
                    .mScope = kAudioObjectPropertyScopeGlobal,
                    .mElement = kAudioObjectPropertyElementMain,
                };
                UInt32 size = sizeof(rate);
                AudioObjectSetPropertyData(deviceID, &rateAddress, 0, nullptr, size, &rate);
            }

            AudioDeviceIOProcID procID = nullptr;
            status = AudioDeviceCreateIOProcID(deviceID, tap_io_proc, (__bridge void*)tap, &procID);
            if (status != noErr) {
                AudioHardwareDestroyAggregateDevice(deviceID);
                AudioHardwareDestroyProcessTap(tapID);
                set_error([NSString stringWithFormat:@"Failed to create IOProc (%d)", status], errorOut);
                return nullptr;
            }

            tap.procID = procID;

            status = AudioDeviceStart(deviceID, procID);
            if (status != noErr) {
                AudioDeviceDestroyIOProcID(deviceID, procID);
                AudioHardwareDestroyAggregateDevice(deviceID);
                AudioHardwareDestroyProcessTap(tapID);
                set_error([NSString stringWithFormat:@"Failed to start device (%d)", status], errorOut);
                return nullptr;
            }

            tap.running = YES;

            return (__bridge_retained void *)tap;
        } else {
            set_error(@"System audio capture requires macOS 14.2 or later", errorOut);
            return nullptr;
        }
    }
}

extern "C" void system_audio_tap_stop(void* handle) {
    if (handle == nullptr) {
        return;
    }

    @autoreleasepool {
        SystemAudioTap* tap = (__bridge_transfer SystemAudioTap*)handle;
        if (tap == nil) {
            return;
        }

        if (tap.isRunning && tap.procID != nullptr) {
            AudioDeviceStop(tap.deviceID, tap.procID);
            tap.running = NO;
        }

        if (tap.procID != nullptr) {
            AudioDeviceDestroyIOProcID(tap.deviceID, tap.procID);
            tap.procID = nullptr;
        }

        if (tap.deviceID != kAudioObjectUnknown) {
            AudioHardwareDestroyAggregateDevice(tap.deviceID);
            tap.deviceID = kAudioObjectUnknown;
        }

        if (@available(macOS 14.2, *)) {
            if (tap.tapID != kAudioObjectUnknown) {
                AudioHardwareDestroyProcessTap(tap.tapID);
                tap.tapID = kAudioObjectUnknown;
            }
        }
    }
}

extern "C" void system_audio_tap_free_error(char* ptr) {
    if (ptr != nullptr) {
        free(ptr);
    }
}
