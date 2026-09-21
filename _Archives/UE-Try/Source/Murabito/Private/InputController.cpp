#include "InputController.h"

#include "CameraRig.h"
#include "EnhancedInputComponent.h"
#include "EnhancedInputSubsystems.h"
#include "Engine/LocalPlayer.h"
#include "InputAction.h"
#include "InputMappingContext.h"
#include "MurabitoLog.h"

AInputController::AInputController()
{
	bShowMouseCursor = true;
}

ACameraRig* AInputController::Rig() const
{
	return Cast<ACameraRig>(GetPawn());
}

void AInputController::SetupInputComponent()
{
	Super::SetupInputComponent();

	UEnhancedInputComponent* Input = Cast<UEnhancedInputComponent>(InputComponent);
	if (!Input)
	{
		UE_LOG(LogMurabito, Warning, TEXT("InputController: input component is not an EnhancedInputComponent; camera controls are off"));
		return;
	}

	auto Bind = [this, Input](UInputAction* Action, const TCHAR* Slot, ETriggerEvent Event, void (AInputController::*Handler)(const FInputActionValue&))
	{
		if (!Action)
		{
			UE_LOG(LogMurabito, Warning, TEXT("InputController: %s is not set; set it in the controller Blueprint"), Slot);
			return;
		}
		Input->BindAction(Action, Event, this, Handler);
	};
	Bind(PanAction, TEXT("PanAction"), ETriggerEvent::Triggered, &AInputController::OnPan);
	Bind(ZoomAction, TEXT("ZoomAction"), ETriggerEvent::Triggered, &AInputController::OnZoom);
	Bind(DragPanAction, TEXT("DragPanAction"), ETriggerEvent::Started, &AInputController::OnDragStarted);
	if (DragPanAction)
	{
		Input->BindAction(DragPanAction, ETriggerEvent::Completed, this, &AInputController::OnDragCompleted);
	}
}

void AInputController::BeginPlay()
{
	Super::BeginPlay();

	if (!CameraControls)
	{
		UE_LOG(LogMurabito, Warning, TEXT("InputController: CameraControls is not set; set it in the controller Blueprint"));
	}
	else if (UEnhancedInputLocalPlayerSubsystem* Subsystem = ULocalPlayer::GetSubsystem<UEnhancedInputLocalPlayerSubsystem>(GetLocalPlayer()))
	{
		Subsystem->AddMappingContext(CameraControls, 0);
	}

	FInputModeGameAndUI Mode;
	Mode.SetHideCursorDuringCapture(false);
	Mode.SetLockMouseToViewportBehavior(EMouseLockMode::DoNotLock);
	SetInputMode(Mode);
}

void AInputController::OnPan(const FInputActionValue& Value)
{
	if (ACameraRig* R = Rig())
	{
		R->AddPan(Value.Get<FVector2D>());
	}
}

void AInputController::OnZoom(const FInputActionValue& Value)
{
	if (ACameraRig* R = Rig())
	{
		R->AddZoom(Value.Get<float>());
	}
}

void AInputController::OnDragStarted(const FInputActionValue& Value)
{
	float X, Y;
	if (GetMousePosition(X, Y))
	{
		bDragging = true;
		LastMouse = FVector2D(X, Y);
	}
}

void AInputController::OnDragCompleted(const FInputActionValue& Value)
{
	bDragging = false;
}

void AInputController::PlayerTick(float DeltaTime)
{
	Super::PlayerTick(DeltaTime);
	UpdateDrag();
	UpdateEdgeScroll();
}

void AInputController::UpdateDrag()
{
	ACameraRig* R = Rig();
	float X, Y;
	if (!bDragging || !R || !GetMousePosition(X, Y))
	{
		return;
	}
	const FVector2D Mouse(X, Y);
	R->AddDrag(Mouse - LastMouse);
	LastMouse = Mouse;
}

void AInputController::UpdateEdgeScroll()
{
	ACameraRig* R = Rig();
	float X, Y;
	if (!R || bDragging || EdgeScrollMargin <= 0.f || !GetMousePosition(X, Y))
	{
		return;
	}
	int32 SizeX, SizeY;
	GetViewportSize(SizeX, SizeY);
	FVector2D Dir = FVector2D::ZeroVector;
	if (X <= EdgeScrollMargin) Dir.X -= 1.0;
	if (X >= SizeX - 1 - EdgeScrollMargin) Dir.X += 1.0;
	if (Y <= EdgeScrollMargin) Dir.Y += 1.0;
	if (Y >= SizeY - 1 - EdgeScrollMargin) Dir.Y -= 1.0;
	if (!Dir.IsZero())
	{
		R->AddPan(Dir);
	}
}
