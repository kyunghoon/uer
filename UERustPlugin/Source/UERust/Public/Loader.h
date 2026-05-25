#pragma once

#include "CoreMinimal.h"

#if WITH_EDITOR
#  include "IDirectoryWatcher.h"
#endif // WITH_EDITOR

DECLARE_LOG_CATEGORY_EXTERN(LogRust, Log, All);

struct LoaderImpl;
struct UERustRsApi;

class UERustLoader {
public:
    UERustLoader(TCHAR const* ModuleDir, TCHAR const* LibraryName, TCHAR const* LibEntryPoint);

#if WITH_EDITOR
    void OnProjectDirectoryChanged(const TArray<FFileChangeData>& Data);
#endif // WITH_EDITOR
    void LoadDll(FString const& Path);

    UERustRsApi& RsApi() const;
private:
    FString LibraryFileName;
    FString LibraryEntryPoint;
    FDelegateHandle WatcherHandle;
    void* Handle;
    FString TargetPath;
    TSharedPtr<LoaderImpl, ESPMode::ThreadSafe> _impl;
};