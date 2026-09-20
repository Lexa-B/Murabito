#include "CameraRig.h"

#include "Camera/CameraComponent.h"
#include "Engine/World.h"
#include "GameFramework/SpringArmComponent.h"

ACameraRig::ACameraRig()
{
	PrimaryActorTick.bCanEverTick = true;

	Pivot = CreateDefaultSubobject<USceneComponent>(TEXT("Pivot"));
	RootComponent = Pivot;

	Arm = CreateDefaultSubobject<USpringArmComponent>(TEXT("Arm"));
	Arm->SetupAttachment(Pivot);
	Arm->bDoCollisionTest = false;
	Arm->bEnableCameraLag = false;
	Arm->bUsePawnControlRotation = false;

	Camera = CreateDefaultSubobject<UCameraComponent>(TEXT("Camera"));
	Camera->SetupAttachment(Arm, USpringArmComponent::SocketName);
}

void ACameraRig::BeginPlay()
{
	Super::BeginPlay();

	// Keep only the spawn yaw: the view faces the way the PlayerStart faces, and never tilts or rolls.
	SetActorRotation(FRotator(0.f, GetActorRotation().Yaw, 0.f));

	Zoom01 = TargetZoom01 = FMath::Clamp(StartZoom, 0.f, 1.f);
	ApplyZoom();

	if (const TOptional<double> Ground = GroundHeight())
	{
		SetActorLocation(FVector(GetActorLocation().X, GetActorLocation().Y, Ground.GetValue()));
		bHasGround = true;
	}
}

FZoomRange ACameraRig::ZoomRange() const
{
	FZoomRange Range;
	Range.NearArm = NearArm;
	Range.FarArm = FarArm;
	Range.NearPitch = NearPitch;
	Range.FarPitch = FarPitch;
	return Range;
}

void ACameraRig::ApplyZoom()
{
	const FZoomRange Range = ZoomRange();
	Arm->TargetArmLength = CameraMath::ArmLength(Range, Zoom01);
	Arm->SetRelativeRotation(FRotator(CameraMath::Pitch(Range, Zoom01), 0.f, 0.f));
}

TOptional<double> ACameraRig::GroundHeight() const
{
	const FVector Here = GetActorLocation();
	const FVector Start(Here.X, Here.Y, Here.Z + GroundTraceReach);
	const FVector End(Here.X, Here.Y, Here.Z - GroundTraceReach);

	FCollisionQueryParams Params(SCENE_QUERY_STAT(CameraRigGround), /*bTraceComplex=*/ false, this);
	FHitResult Hit;
	if (GetWorld()->LineTraceSingleByChannel(Hit, Start, End, ECC_Visibility, Params))
	{
		return Hit.ImpactPoint.Z;
	}
	return {};
}

void ACameraRig::AddPan(const FVector2D& ScreenDir) { PendingScreenPan += ScreenDir; }
void ACameraRig::AddDrag(const FVector2D& PixelDelta) { PendingDrag += PixelDelta; }

void ACameraRig::AddZoom(float Steps)
{
	// Wheel up (positive) zooms in, toward 0.
	TargetZoom01 = FMath::Clamp(TargetZoom01 - Steps * ZoomStep, 0.f, 1.f);
}

void ACameraRig::Tick(float DeltaSeconds)
{
	Super::Tick(DeltaSeconds);

	Zoom01 = FMath::FInterpTo(Zoom01, TargetZoom01, DeltaSeconds, ZoomSmoothing);
	ApplyZoom();

	const float Speed = CameraMath::PanSpeed(PanSpeed, ZoomRange(), Zoom01);
	const float CmPerPixel = DragCmPerPixelPerArm * Arm->TargetArmLength;
	const FVector2D Local = CameraMath::ScreenDirToLocal(PendingScreenPan.GetClampedToMaxSize(1.0)) * Speed * DeltaSeconds
		+ CameraMath::DragToLocal(PendingDrag, CmPerPixel);
	PendingScreenPan = FVector2D::ZeroVector;
	PendingDrag = FVector2D::ZeroVector;

	const FVector2D Move = CameraMath::LocalToWorld(Local, GetActorRotation().Yaw);
	FVector Location = GetActorLocation() + FVector(Move.X, Move.Y, 0.0);

	if (const TOptional<double> Ground = GroundHeight())
	{
		// Snap the first time ground is found, then ease so the view doesn't jolt over bumps.
		Location.Z = bHasGround ? FMath::FInterpTo(Location.Z, Ground.GetValue(), DeltaSeconds, GroundFollowSpeed) : Ground.GetValue();
		bHasGround = true;
	}
	SetActorLocation(Location);
}
