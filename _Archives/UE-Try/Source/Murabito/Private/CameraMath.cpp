#include "CameraMath.h"

namespace CameraMath
{
	float ArmLength(const FZoomRange& Range, float Zoom01)
	{
		return FMath::Lerp(Range.NearArm, Range.FarArm, FMath::Clamp(Zoom01, 0.f, 1.f));
	}

	float Pitch(const FZoomRange& Range, float Zoom01)
	{
		return FMath::Lerp(Range.NearPitch, Range.FarPitch, FMath::Clamp(Zoom01, 0.f, 1.f));
	}

	float PanSpeed(float BaseSpeed, const FZoomRange& Range, float Zoom01)
	{
		return BaseSpeed * ArmLength(Range, Zoom01) / Range.NearArm;
	}

	FVector2D ScreenDirToLocal(const FVector2D& ScreenDir)
	{
		return FVector2D(ScreenDir.Y, ScreenDir.X);
	}

	FVector2D DragToLocal(const FVector2D& PixelDelta, float CmPerPixel)
	{
		return FVector2D(PixelDelta.Y * CmPerPixel, -PixelDelta.X * CmPerPixel);
	}

	FVector2D LocalToWorld(const FVector2D& Local, float YawDegrees)
	{
		const FVector World = FRotator(0.f, YawDegrees, 0.f).RotateVector(FVector(Local.X, Local.Y, 0.f));
		return FVector2D(World.X, World.Y);
	}
}
