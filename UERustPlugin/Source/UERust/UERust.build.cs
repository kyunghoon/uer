using UnrealBuildTool;
using System.IO;

public class UERust : ModuleRules
{
    public UERust(ReadOnlyTargetRules Target) : base(Target)
    {
        PrivateDefinitions.Add("UERUST_PACKAGE=1");

        PCHUsage = ModuleRules.PCHUsageMode.UseExplicitOrSharedPCHs;

        PublicDependencyModuleNames.AddRange(new string[] { "PCG" });
        PrivateDependencyModuleNames.AddRange(new string[] { "Core","CoreUObject","Engine" });


        PublicIncludePaths.Add(ModuleDirectory);
        PublicIncludePaths.AddRange(new string[] {  });
        PrivateIncludePaths.AddRange(new string[] {  });

        if (Target.Platform == UnrealTargetPlatform.Android) {
            string AndroidPath = System.IO.Path.Combine(ModuleDirectory, UnrealTargetPlatform.Android.ToString(), "arm64-v8a");
            AdditionalPropertiesForReceipt.Add("AndroidPlugin", System.IO.Path.Combine(ModuleDirectory, "BaseAPL.xml"));
            PublicAdditionalLibraries.Add(System.IO.Path.Combine(AndroidPath, "libUERust.so"));
            RuntimeDependencies.Add(System.IO.Path.Combine(AndroidPath, "libUERust.so"));
        } else if (Target.Platform == UnrealTargetPlatform.Win64) {
            string WindowPath = System.IO.Path.Combine(ModuleDirectory, UnrealTargetPlatform.Win64.ToString());
            PublicAdditionalLibraries.Add(System.IO.Path.Combine(WindowPath, "UERust.dll.lib"));
            PublicDelayLoadDLLs.Add("UERust.dll");
            RuntimeDependencies.Add(System.IO.Path.Combine(WindowPath, "UERust.dll"));
        } else if (Target.Platform == UnrealTargetPlatform.Mac) {
            string MacPath = System.IO.Path.Combine(ModuleDirectory, UnrealTargetPlatform.Mac.ToString());
            PublicAdditionalLibraries.Add(System.IO.Path.Combine(MacPath, "libUERust.dylib"));
            RuntimeDependencies.Add(System.IO.Path.Combine(MacPath, "libUERust.dylib"));
        }
    }
}