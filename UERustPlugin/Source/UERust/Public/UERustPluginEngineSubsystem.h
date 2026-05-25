#pragma once

#include "CoreMinimal.h"
#include "Subsystems/EngineSubsystem.h"
#include "bindings.h"
#include "UERustPluginEngineSubsystem.generated.h"

UCLASS()
class UERUST_API UUERustPluginEngineSubsystem : public UEngineSubsystem {
    GENERATED_BODY()
public:
    TFunction<void(UUERustPluginEngineSubsystem&, bool)> OnLoaded;
    TFunction<Return(uint16_t, Argument const*, size_t)> OnInvoke;

public:
    void SetOnLoaded(TFunction<void(UUERustPluginEngineSubsystem&, bool)> const& onLoaded);
    Return RInvoke(uint16_t rmethodId, Argument const* args, size_t len) const;
};