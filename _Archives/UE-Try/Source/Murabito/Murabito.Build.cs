using UnrealBuildTool;

public class Murabito : ModuleRules
{
	public Murabito(ReadOnlyTargetRules Target) : base(Target)
	{
		PCHUsage = PCHUsageMode.UseExplicitOrSharedPCHs;

		PublicDependencyModuleNames.AddRange(new string[]
		{
			"Core", "CoreUObject", "Engine", "InputCore", "EnhancedInput",
		});

		// Tests read the project's configured default game mode.
		PrivateDependencyModuleNames.Add("EngineSettings");
	}
}
