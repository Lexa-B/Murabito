#pragma once

#include "CoreMinimal.h"
#include "GameFramework/PlayerController.h"
#include "InputController.generated.h"

class ACameraRig;
class UInputAction;
class UInputMappingContext;
struct FInputActionValue;

/**
 * Keyboard and mouse input for the camera rig. The input assets are set in a Blueprint child
 * (BP_InputController); an empty slot logs a warning and that control does nothing.
 */
UCLASS(Blueprintable)
class MURABITO_API AInputController : public APlayerController
{
	GENERATED_BODY()

public:
	AInputController();

	/** Mapping context with the camera's key bindings (IMC_Camera). */
	UPROPERTY(EditDefaultsOnly, BlueprintReadOnly, Category = "Input") TObjectPtr<UInputMappingContext> CameraControls;
	/** 2D axis: X right, Y up the screen (IA_Pan). */
	UPROPERTY(EditDefaultsOnly, BlueprintReadOnly, Category = "Input") TObjectPtr<UInputAction> PanAction;
	/** 1D axis: positive zooms in (IA_Zoom). */
	UPROPERTY(EditDefaultsOnly, BlueprintReadOnly, Category = "Input") TObjectPtr<UInputAction> ZoomAction;
	/** Button: held to drag the view with the mouse (IA_DragPan). */
	UPROPERTY(EditDefaultsOnly, BlueprintReadOnly, Category = "Input") TObjectPtr<UInputAction> DragPanAction;

	/** Pixels from the window edge that start edge scrolling; 0 turns it off. */
	UPROPERTY(EditAnywhere, BlueprintReadOnly, Category = "Input") float EdgeScrollMargin = 8.f;

protected:
	virtual void BeginPlay() override;
	virtual void SetupInputComponent() override;
	virtual void PlayerTick(float DeltaTime) override;

private:
	void OnPan(const FInputActionValue& Value);
	void OnZoom(const FInputActionValue& Value);
	void OnDragStarted(const FInputActionValue& Value);
	void OnDragCompleted(const FInputActionValue& Value);
	void UpdateEdgeScroll();
	void UpdateDrag();
	ACameraRig* Rig() const;

	bool bDragging = false;
	FVector2D LastMouse = FVector2D::ZeroVector;
};
