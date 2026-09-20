#pragma once

#include "CoreMinimal.h"

/** Zoom limits. Zoom01 = 0 is the near limit, 1 the far. Pitch is in degrees; negative looks down. */
struct FZoomRange
{
	float NearArm = 1500.f;
	float FarArm = 12000.f;
	float NearPitch = -45.f;
	float FarPitch = -75.f;
};

/**
 * Pure helpers for the overhead camera, kept apart from the actor so they can be tested.
 * "Local" means the rig's own frame: X forward, Y right. World is UE's X forward, Y right, Z up.
 */
namespace CameraMath
{
	MURABITO_API float ArmLength(const FZoomRange& Range, float Zoom01);

	/** Tilt follows zoom: oblique when close, steep when far. */
	MURABITO_API float Pitch(const FZoomRange& Range, float Zoom01);

	/** Pan speed in cm/s: BaseSpeed at the near limit, scaled by arm length. */
	MURABITO_API float PanSpeed(float BaseSpeed, const FZoomRange& Range, float Zoom01);

	/** Screen direction (X right, Y up the screen) -> local (X forward, Y right). */
	MURABITO_API FVector2D ScreenDirToLocal(const FVector2D& ScreenDir);

	/** Mouse drag in pixels (screen coords, +Y down) -> local offset that keeps the ground under the cursor. */
	MURABITO_API FVector2D DragToLocal(const FVector2D& PixelDelta, float CmPerPixel);

	/** Local XY -> world XY for a rig turned YawDegrees about Z. */
	MURABITO_API FVector2D LocalToWorld(const FVector2D& Local, float YawDegrees);
}
