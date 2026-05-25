#include "UERustModule.h"
#include "bindings.h"
#include "UERustPluginEngineSubsystem.h"

#if WITH_EDITOR
#include "Editor.h"
#endif // WITH_EDITOR

IMPLEMENT_MODULE(FUERustModule, UERust);

FUERustModule::FUERustModule()
    : _Loader(TEXT("Plugins/UERustPlugin/Source/UERust/"), TEXT("UERust"), TEXT("GetUERustRsApi_0"))
    , _Subsystem(nullptr)
{}

void FUERustModule::StartupModule()
{
    FCoreDelegates::OnPostEngineInit.AddLambda([this]() {
        if (GEngine) {
            if (UUERustPluginEngineSubsystem* Subsystem = GEngine->GetEngineSubsystem<UUERustPluginEngineSubsystem>()) {
                _Subsystem = Subsystem;
            }
        }
    });
}

void FUERustModule::ShutdownModule() { }

FUERustModule& FUERustModule::Get() { 
    static TOptional<FUERustModule*> MODULE = {};
    if (!MODULE.IsSet()) {
        MODULE = TOptional((FUERustModule*)FModuleManager::Get().GetModule("UERust"));
        check(MODULE != nullptr);
    };
    return *MODULE.GetValue();
}

Return FUERustModule::Invoke(uint16_t methodId, Argument const* args, size_t len) const {
    if (_Subsystem == nullptr || !_Subsystem->OnInvoke) return Return { .is_some = false };
    return _Subsystem->OnInvoke(methodId, args, len);
}

Return FUERustModule::RInvoke(uint16_t rmethodId, Argument const* args, size_t len) const {
    return _Loader.RsApi().rinvoke(rmethodId, args, len);
}

void FUERustModule::NotifyLoaded(bool isReload) {
    if (_Subsystem == nullptr || !_Subsystem->OnLoaded) return;
    _Subsystem->OnLoaded(*_Subsystem, isReload);
}

UERustRsApi& GetUERustRsApi_0() {
    return FUERustModule::Get()._Loader.RsApi();
}