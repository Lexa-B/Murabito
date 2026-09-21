#pragma once

#include "CoreMinimal.h"
#include "CameraMath.h"
#include "GameFramework/Pawn.h"
#include "CameraRig.generated.h"

class UCameraComponent;
class USpringArmComponent;

/**
 * Overhead camera: a pivot on the ground, a spring arm and a camera. Pans over the ground and zooms
 * along the arm; tilt follows zoom (steep far out, oblique close in). The pivot's height follows the
 * ground under it. It keeps the yaw it spawned with (the PlayerStart's) and never rotates.
 * Subclasses APawn only because the engine possesses pawns.
 */
UCLASS(Blueprintable)
class MURABITO_API ACameraRig : public APawn
{
	GENERATED_BODY()

public:
	ACameraRig();

	virtual void Tick(float DeltaSeconds) override;

	/** Screen-relative pan for this frame: X right, Y up the screen, each in [-1, 1]. */
	void AddPan(const FVector2D& ScreenDir);

	/** Mouse drag this frame, in pixels (screen coords, +Y down). */
	void AddDrag(const FVector2D& PixelDelta);

	/** Wheel steps: positive zooms in. */
	void AddZoom(float Steps);

	/** Arm length when fully zoomed in, in cm. */
	UPROPERTY(EditAnywhere, BlueprintReadOnly, Category = "Camera|Zoom") float NearArm = 1500.f;
	/** Arm length when fully zoomed out, in cm. */
	UPROPERTY(EditAnywhere, BlueprintReadOnly, Category = "Camera|Zoom") float FarArm = 12000.f;
	/** Tilt when fully zoomed in, in degrees (negative looks down). */
	UPROPERTY(EditAnywhere, BlueprintReadOnly, Category = "Camera|Zoom") float NearPitch = -45.f;
	/** Tilt when fully zoomed out, in degrees (negative looks down). */
	UPROPERTY(EditAnywhere, BlueprintReadOnly, Category = "Camera|Zoom") float FarPitch = -75.f;
	/** Fraction of the zoom range per wheel step. */
	UPROPERTY(EditAnywhere, BlueprintReadOnly, Category = "Camera|Zoom", meta = (ClampMin = "0.01", ClampMax = "1")) float ZoomStep = 0.08f;
	/** How quickly zoom eases to its target (higher is snappier). */
	UPROPERTY(EditAnywhere, BlueprintReadOnly, Category = "Camera|Zoom") float ZoomSmoothing = 10.f;
	/** Zoom at the start: 0 is fully in, 1 fully out. */
	UPROPERTY(EditAnywhere, BlueprintReadOnly, Category = "Camera|Zoom", meta = (ClampMin = "0", ClampMax = "1")) float StartZoom = 0.6f;

	/** Pan speed at the near limit, in cm/s; scales up with zoom. */
	UPROPERTY(EditAnywhere, BlueprintReadOnly, Category = "Camera|Pan") float PanSpeed = 1500.f;
	/** Mouse-drag pan: cm moved per pixel, per cm of arm length. */
	UPROPERTY(EditAnywhere, BlueprintReadOnly, Category = "Camera|Pan") float DragCmPerPixelPerArm = 0.0012f;

	/** How quickly the pivot's height eases to the ground under it (higher is snappier). */
	UPROPERTY(EditAnywhere, BlueprintReadOnly, Category = "Camera|Ground") float GroundFollowSpeed = 6.f;
	/** How far above and below the pivot to look for ground, in cm. */
	UPROPERTY(EditAnywhere, BlueprintReadOnly, Category = "Camera|Ground") float GroundTraceReach = 100000.f;

	UPROPERTY(VisibleAnywhere, BlueprintReadOnly, Category = "Camera") TObjectPtr<USceneComponent> Pivot;
	UPROPERTY(VisibleAnywhere, BlueprintReadOnly, Category = "Camera") TObjectPtr<USpringArmComponent> Arm;
	UPROPERTY(VisibleAnywhere, BlueprintReadOnly, Category = "Camera") TObjectPtr<UCameraComponent> Camera;

protected:
	virtual void BeginPlay() override;

private:
	FZoomRange ZoomRange() const;
	void ApplyZoom();
	/** Height of the ground under the pivot, if the trace hits anything. */
	TOptional<double> GroundHeight() const;

	float Zoom01 = 0.6f;
	float TargetZoom01 = 0.6f;
	bool bHasGround = false;
	FVector2D PendingScreenPan = FVector2D::ZeroVector;
	FVector2D PendingDrag = FVector2D::ZeroVector;
};
