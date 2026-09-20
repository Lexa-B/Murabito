using UnrealBuildTool;

public class MurabitoEditorTarget : TargetRules
{
	public MurabitoEditorTarget(TargetInfo Target) : base(Target)
	{
		Type = TargetType.Editor;
		DefaultBuildSettings = BuildSettingsVersion.V7;
		IncludeOrderVersion = EngineIncludeOrderVersion.Unreal5_8;
		ExtraModuleNames.Add("Murabito");
	}
}
