#include "Misc/AutomationTest.h"

#include "CameraMath.h"

#if WITH_DEV_AUTOMATION_TESTS

namespace
{
	constexpr EAutomationTestFlags CameraMathTestFlags =
		EAutomationTestFlags_ApplicationContextMask | EAutomationTestFlags::EngineFilter;
}

IMPLEMENT_SIMPLE_AUTOMATION_TEST(FCameraMathZoomTiltTest, "Murabito.CameraMath.ZoomTilt", CameraMathTestFlags)
bool FCameraMathZoomTiltTest::RunTest(const FString& Parameters)
{
	const FZoomRange Z;
	TestEqual(TEXT("near arm"), CameraMath::ArmLength(Z, 0.f), 1500.f, 0.01f);
	TestEqual(TEXT("far arm"), CameraMath::ArmLength(Z, 1.f), 12000.f, 0.01f);
	TestEqual(TEXT("mid arm"), CameraMath::ArmLength(Z, 0.5f), 6750.f, 0.01f);
	TestEqual(TEXT("near pitch is oblique"), CameraMath::Pitch(Z, 0.f), -45.f, 0.01f);
	TestEqual(TEXT("far pitch is steep"), CameraMath::Pitch(Z, 1.f), -75.f, 0.01f);
	TestEqual(TEXT("mid pitch"), CameraMath::Pitch(Z, 0.5f), -60.f, 0.01f);
	TestEqual(TEXT("zoom clamps low"), CameraMath::ArmLength(Z, -1.f), 1500.f, 0.01f);
	TestEqual(TEXT("zoom clamps high"), CameraMath::Pitch(Z, 2.f), -75.f, 0.01f);
	return true;
}

IMPLEMENT_SIMPLE_AUTOMATION_TEST(FCameraMathPanMappingTest, "Murabito.CameraMath.PanMapping", CameraMathTestFlags)
bool FCameraMathPanMappingTest::RunTest(const FString& Parameters)
{
	// Screen up -> forward (+X); screen right -> right (+Y), in the rig's own frame.
	TestTrue(TEXT("up is forward"), CameraMath::ScreenDirToLocal(FVector2D(0, 1)).Equals(FVector2D(1, 0)));
	TestTrue(TEXT("right is right"), CameraMath::ScreenDirToLocal(FVector2D(1, 0)).Equals(FVector2D(0, 1)));

	// Dragging keeps the ground under the cursor: mouse right (+px X) moves the rig left (-Y);
	// mouse down (+px Y, screen coords) moves it forward (+X).
	TestTrue(TEXT("drag right"), CameraMath::DragToLocal(FVector2D(10, 0), 2.f).Equals(FVector2D(0, -20)));
	TestTrue(TEXT("drag down"), CameraMath::DragToLocal(FVector2D(0, 10), 2.f).Equals(FVector2D(20, 0)));

	// Pan speed scales with arm length: 8x the base at 8x the distance.
	const FZoomRange Z;
	TestEqual(TEXT("near speed is base"), CameraMath::PanSpeed(1000.f, Z, 0.f), 1000.f, 0.01f);
	TestEqual(TEXT("far speed scales"), CameraMath::PanSpeed(1000.f, Z, 1.f), 8000.f, 0.01f);
	return true;
}

IMPLEMENT_SIMPLE_AUTOMATION_TEST(FCameraMathLocalToWorldTest, "Murabito.CameraMath.LocalToWorld", CameraMathTestFlags)
bool FCameraMathLocalToWorldTest::RunTest(const FString& Parameters)
{
	// Facing +X (yaw 0): local and world agree.
	TestTrue(TEXT("yaw 0 forward"), CameraMath::LocalToWorld(FVector2D(1, 0), 0.f).Equals(FVector2D(1, 0), 1e-4));
	// Facing +Y (yaw 90): forward is +Y, right is -X.
	TestTrue(TEXT("yaw 90 forward"), CameraMath::LocalToWorld(FVector2D(1, 0), 90.f).Equals(FVector2D(0, 1), 1e-4));
	TestTrue(TEXT("yaw 90 right"), CameraMath::LocalToWorld(FVector2D(0, 1), 90.f).Equals(FVector2D(-1, 0), 1e-4));
	// Length is kept.
	TestEqual(TEXT("length kept"), CameraMath::LocalToWorld(FVector2D(3, 4), 37.f).Size(), 5.0, 1e-4);
	return true;
}

#endif
