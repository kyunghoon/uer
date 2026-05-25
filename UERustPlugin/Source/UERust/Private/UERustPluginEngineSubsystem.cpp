#include "UERustPluginEngineSubsystem.h"

void UUERustPluginEngineSubsystem::SetOnLoaded(TFunction<void(UUERustPluginEngineSubsystem&, bool)> const& onLoaded) {
    OnLoaded = onLoaded;
    if (OnLoaded) {
        OnLoaded(*this, false);
    }
}

Return UUERustPluginEngineSubsystem::RInvoke(uint16_t rmethodId, Argument const* args, size_t len) const {
    return GetUERustRsApi_0().rinvoke(rmethodId, args, len);
}