#include "Misc/AutomationTest.h"

#include "CameraRig.h"
#include "GameMapsSettings.h"
#include "GameRules.h"
#include "InputController.h"
#include "UObject/SoftObjectPath.h"

#if WITH_DEV_AUTOMATION_TESTS

namespace
{
	constexpr EAutomationTestFlags GameRulesTestFlags =
		EAutomationTestFlags_ApplicationContextMask | EAutomationTestFlags::EngineFilter;
}

IMPLEMENT_SIMPLE_AUTOMATION_TEST(FGameRulesIsProjectDefaultTest, "Murabito.GameRules.IsProjectDefault", GameRulesTestFlags)
bool FGameRulesIsProjectDefaultTest::RunTest(const FString& Parameters)
{
	// AGameRules itself, or a Blueprint child of it.
	const FString Configured = UGameMapsSettings::GetGlobalDefaultGameMode();
	const UClass* Resolved = FSoftClassPath(Configured).TryLoadClass<AGameModeBase>();
	TestTrue(FString::Printf(TEXT("default game mode '%s' is AGameRules or a child of it"), *Configured),
		Resolved != nullptr && Resolved->IsChildOf(AGameRules::StaticClass()));
	return true;
}

IMPLEMENT_SIMPLE_AUTOMATION_TEST(FGameRulesDefaultClassesTest, "Murabito.GameRules.DefaultClasses", GameRulesTestFlags)
bool FGameRulesDefaultClassesTest::RunTest(const FString& Parameters)
{
	const AGameRules* Rules = GetDefault<AGameRules>();
	TestTrue(TEXT("default pawn is the camera rig"), Rules->DefaultPawnClass == ACameraRig::StaticClass());
	TestTrue(TEXT("player controller is the input controller"), Rules->PlayerControllerClass == AInputController::StaticClass());
	return true;
}

#endif
