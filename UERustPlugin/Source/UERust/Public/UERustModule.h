// Copyright Epic Games, Inc. All Rights Reserved.

#pragma once

#include "HAL/PlatformCrt.h"
#include "Modules/ModuleInterface.h"
#include "Misc/EngineVersionComparison.h"
#include "Loader.h"
#include "bindings.h"

class UUERustPluginEngineSubsystem;

class FUERustModule : public IModuleInterface
{
    friend UERustRsApi& GetUERustRsApi_0();
private:
    UERustLoader _Loader;
	UUERustPluginEngineSubsystem* _Subsystem;

PACKAGE_SCOPE:
public:

	FUERustModule();

	virtual ~FUERustModule() {}

	// IModuleInterface
	virtual void StartupModule() override;
	virtual void ShutdownModule() override;
	virtual bool SupportsDynamicReloading() override { return false; }
	virtual bool SupportsAutomaticShutdown() override { return false; }

	Return Invoke(uint16_t methodId, Argument const* args, size_t len) const;
	Return RInvoke(uint16_t rmethodId, Argument const* args, size_t len) const;
	void NotifyLoaded(bool isReload);

    static FUERustModule& Get();
};

#if !UE_VERSION_NEWER_THAN_OR_EQUAL(5,7,0)
#  if UE_ENABLE_INCLUDE_ORDER_DEPRECATED_IN_5_2
#    include "CoreMinimal.h"
#  endif
#endif