#include "Loader.h"

#include "CoreMinimal.h"
#include "Containers/Array.h"
#include "Misc/Paths.h"
#if WITH_EDITOR
#  include "HAL/PlatformProcess.h"
#  include "IDirectoryWatcher.h"
#  include "LevelEditor.h"
#  include "Editor/EditorEngine.h"
#  include "DirectoryWatcherModule.h"
#else
#  include "HAL/PlatformFilemanager.h"
#  include "GenericPlatform/GenericPlatformFile.h"
#endif // WITH_EDITOR

#include "UERustModule.h"

#define DLL_DEBUG 0

static FString PluginLibraryPath(TCHAR const* ModuleDir) {
#if PLATFORM_ANDROID
	return FPaths::Combine(*FPaths::ConvertRelativePathToFull(FPaths::ProjectDir()), ModuleDir, TEXT("Android/arm64-v8a"));
#elif PLATFORM_MAC
	return FPaths::Combine(*FPaths::ConvertRelativePathToFull(FPaths::ProjectDir()), ModuleDir, TEXT("Mac"));
#elif PLATFORM_WINDOWS
	return FPaths::Combine(*FPaths::ConvertRelativePathToFull(FPaths::ProjectDir()), ModuleDir, TEXT("Win64"));
#else
#  error("unsupported platform")
#endif
}



struct LoaderImpl {
    UERustGetRsApiFn GetRsApiFn;
    TOptional<UERustRsApi> RsApi;
};

UERustLoader::UERustLoader(TCHAR const* ModuleDir, TCHAR const* LibraryName, TCHAR const* LibEntryPoint)
    : _impl(MakeShared<LoaderImpl>()) {
#if PLATFORM_LINUX || PLATFORM_ANDROID
	LibraryFileName = FString::Printf(TEXT("lib%s.so"), LibraryName);
#elif PLATFORM_WINDOWS
    LibraryFileName = FString::Printf(TEXT("%s.dll"), LibraryName);
#elif PLATFORM_MAC
    LibraryFileName = FString::Printf(TEXT("lib%s.dylib"), LibraryName);
#else
#  error("unsupported platform")
#endif

    LibraryEntryPoint = FString::Printf(TEXT("%s\0"), LibEntryPoint);

#if WITH_EDITOR
    IDirectoryWatcher* watcher = FModuleManager::LoadModuleChecked<FDirectoryWatcherModule>(TEXT("DirectoryWatcher")).Get();
    check(watcher != nullptr);
    watcher->RegisterDirectoryChangedCallback_Handle(
        *PluginLibraryPath(ModuleDir),
        IDirectoryWatcher::FDirectoryChanged::CreateRaw(this, &UERustLoader::OnProjectDirectoryChanged),
        WatcherHandle, IDirectoryWatcher::WatchOptions::IgnoreChangesInSubtree);

    LoadDll(PluginLibraryPath(ModuleDir) + "/" + LibraryFileName);
#else
    LoadDll(LibraryFileName);
#endif // WITH_EDITOR
}

#if WITH_EDITOR
void UERustLoader::OnProjectDirectoryChanged(const TArray<FFileChangeData>& Data) {
    for (FFileChangeData Changed : Data) {
        FString Filename = FPaths::GetCleanFilename(Changed.Filename);
        const bool ChangedOrAdded = Changed.Action == FFileChangeData::FCA_Added || Changed.Action == FFileChangeData::FCA_Modified;
        if (Filename == LibraryFileName && ChangedOrAdded) {
            LoadDll(Changed.Filename);
            //UE_LOG(LogTemp, Display, TEXT("Hotreload: Rust"));
            break;
        }
    }
}
#endif // WITH_EDITOR

void UERustLoader::LoadDll(FString const& Path) {
    // Loading dlls is a bit tricky, see https://fasterthanli.me/articles/so-you-want-to-live-reload-rust
    // The gist is we can't easily hot reload a dll if the dll uses the thread local storage (TLS).
    // The TLS will prevent the dll from being unloaded even when we call `dlclose`. And `dlopen` will return
    // the pointer to the previously loaded dll.
    // Essentially this means hot reloading will do nothing as we can't unload the currently loaded dll.
    // The workaround for this is give each dll a unique name. Here we append the unix timestamp at
    // the end of the file. That way we can force `dlopen` to load the dll.
    // Please note this is a hack, and this _should_ leak and increase the memory every time you hot reload.
    // This behavior is the same on Linux, Windows and most likely all the other platforms.

    if (DLL_DEBUG) { UE_LOG(LogTemp, Display, TEXT("********** RUST API LOADER - Attempting to load from '%s'"), *Path); }
    FString LocalTargetPath = FString::Printf(TEXT("%s%s-%i"), 
        *FString(FPlatformProcess::UserTempDir()), 
        *LibraryFileName,
        FDateTime::Now().ToUnixTimestamp());

#if WITH_EDITOR
    if (this->Handle != nullptr) {
        if (DLL_DEBUG) { UE_LOG(LogTemp, Display, TEXT("Freeing Dll Handle")); }
        FPlatformProcess::FreeDllHandle(this->Handle);
        this->Handle = nullptr;
        if (!this->TargetPath.IsEmpty()) {
            if (DLL_DEBUG) { UE_LOG(LogTemp, Display, TEXT("Deleting Old Dll File")); }
            if (!FPlatformFileManager::Get().GetPlatformFile().DeleteFile(*this->TargetPath)) {
                UE_LOG(LogTemp, Warning, TEXT("Unable to delete File '%s'"), *this->TargetPath);
            }
        }
    }

    if (DLL_DEBUG) { UE_LOG(LogTemp, Display, TEXT("Deleting Old Dll File")); }
    if (!FPlatformFileManager::Get().GetPlatformFile().CopyFile(*LocalTargetPath, *Path)) {
        UE_LOG(LogTemp, Warning, TEXT("Unable to copy File from '%s' to '%s'"), *Path, *LocalTargetPath);
        return;
    }
    if (DLL_DEBUG) { UE_LOG(LogTemp, Display, TEXT("Getting New Dll Handle")); }
    void* LocalHandle = FPlatformProcess::GetDllHandle(*LocalTargetPath);
#else
    if (DLL_DEBUG) { UE_LOG(LogTemp, Display, TEXT("Getting New Dll Handle")); }
    void* LocalHandle = FPlatformProcess::GetDllHandle(*Path);
#endif // WITH_EDITOR
    if (LocalHandle == nullptr) {
        UE_LOG(LogTemp, Warning, TEXT("Dll open failed"));
        return;
    }

    if (DLL_DEBUG) { UE_LOG(LogTemp, Display, TEXT("Loading Rust Entrypoint '%s'..."), *LibraryEntryPoint); }
    UERustGetRsApiFn GetRsApiFn = (UERustGetRsApiFn)FPlatformProcess::GetDllExport(LocalHandle, *LibraryEntryPoint);
    if (GetRsApiFn == nullptr) {
        UE_LOG(LogTemp, Warning, TEXT("********** UERUST ENTRYPOINT[%s] NOT FOUND **********"), *LibraryEntryPoint);
        FPlatformProcess::FreeDllHandle(LocalHandle);
        return;
    }
    this->Handle = LocalHandle;
    this->TargetPath = LocalTargetPath;
    _impl->GetRsApiFn = GetRsApiFn;
    if (DLL_DEBUG) { UE_LOG(LogTemp, Display, TEXT("Loaded UERust Entrypoint '%p'"), GetRsApiFn); }
    bool was_reloaded = _impl->RsApi.IsSet();
    _impl->RsApi.Emplace(_impl->GetRsApiFn(__uerust_get_capi_0()));
    check(_impl->RsApi.IsSet());

    if (was_reloaded) {
        FUERustModule::Get().NotifyLoaded(true);
    }
}

UERustRsApi& UERustLoader::RsApi() const {
    check(_impl->RsApi.IsSet());
    return _impl->RsApi.GetValue();
}